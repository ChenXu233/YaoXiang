# Unified Type Representation

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution-stage ordering are defined in the main
> body of RFC-039; the positioning of each companion document is described in the
> [directory index](index.md).

## Positioning and Scope

### Covered

Consolidate the multiple parallel type representations currently in use into one, thereby
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero construction in the parser-forward direction** (among the 26
  variants in `ast.rs:427-541`, see 2.2; the “forward” definition excludes reverse bridges and
  AST→AST reconstruction points)
- **A handwritten table of type-name synonyms** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 consolidated
  this into a 15-name parsing table)
- **The parser’s real reverse dependency on the typecheck layer** (2 locations, see 2.4)
- **A production-zero-construction expression variant**, `Expr::FnDef` (**removed: WBS 6.5.1,
  2026-10-30**—the variant definition and all consuming arms were removed, see 2.6)

The concrete deliverables are four items: a disposition table for all 26 variants (5.2), a semantic
classification of `Type::Name` through `NameKind` and elimination of the synonym table (5.3), an
injectable data-flow mechanism through `TypeEnvProbe` (5.4), and build-time type-table gates T1/T2
(5.7; the original T3 was removed following the user decision on 2026-10-30).

### Directory Renaming and the Path of This Article

RFC-039 decision D1 mandates renaming the implementation directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`), **to be completed together with P5/P6, without phasing**. The 50-item
change list and all acceptance greps in this article are **written against the pre-rename
paths**—they constitute the construction baseline for the P6 batches **before the rename batch**.
The directory rename at the end of each batch is a **pure move batch** (C1 zero-diff), committed
separately. The baseline becoming invalid after the rename commit is expected; use git history as
the bisection criterion.

### Not Covered

- **How to perform the directory rename**. `Type` is not moved out of `parser/ast.rs` during this
  phase; only `lower.rs` is added under `types/` as the single conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 according to RFC-039 decision
  D1; the resulting structure is described in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and validator implementation**. The definitions of C1-C6 and the
  three criterion tiers (IR validator / normalized snapshots / corpus differential) are described in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This article references only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This article is a prerequisite for that work, for
  the reasons described below.
- **The global P1-P10 execution sequence**. See the main body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This article defines only the stage breakdown for
  its own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the higher-level master plan: the four-layer model, criterion tiers, cross-document stage
ordering, and the rationale for performing this refactoring now. This article is the construction
plan for **the type-representation convergence workstream within RFC-039**: which variants to
remove, which files to modify, the order of the work, and which gates prevent regression.

Every line number, variant list, and construction-point count in this article was obtained through
static verification of `src/`. Each item is accompanied by `file:line` evidence. **This article
records code facts and the dispositions derived from them, not the execution process.**

To facilitate cross-references with documents in the same batch, the “Target Design” and “Detailed
Design” chapters retain the `5.x` / `6.x` subsection numbering used elsewhere, while the remaining
chapters follow this directory’s established conventions.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(the `pub use` at the head of `src/middle/core/ir.rs` before 6.2.2; that alias has since been
removed and is therefore no longer referenced by line number below). [04-ssa.md](04-ssa.md) will
introduce register type annotations and new instructions into the IR. Unless representation is
consolidated beforehand, SSA type annotations will grow directly on the 26-variant syntactic type
through the `ir::Type` alias, effectively institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree inside a type.
  It is impossible for `Expr` to appear in an IR instruction’s type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Performing SSA type annotations on top of the 26 variants would permanently fossilize the 13 dead
variants and the two syntax-specific variants in the IR contract.

## Current State

Everything in this section is a statement of code fact, not a proposal. Each item includes
`file:line` evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 implemented)**: this section records the state **before** 6.2
> was implemented. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed all four type carriers to carry
> `MonoType`, removed the lossy `From<MonoType> for IrType` bridge and the `ir::Type` re-export
> alias; every statement below concerning `ir::Type` or the lossy bridge is therefore a
> **pre-change** fact, while locations marked “(deleted in 6.2.2)” describe the disposition now in
> effect. See the commit message for task 6.2 for implementation details and measured results.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                               | Causal relationship to type representation                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The same type text has multiple valid AST forms                                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, and the parser selects the latter, type checking must supply a table to translate the former back                                                                                  |
| Bytecode-layer signatures differ from IR-layer signatures                        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias; consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), whereas `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one built-in type name requires changes in multiple places with no checks | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; both locations duplicate the knowledge and do not validate each other                                                                                                                        |
| The parser hard-codes `"Terminates"`                                             | Because the parser cannot determine whether something “is a type” or “is an operator interface,” it embeds type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                                |

**The first three items are representation problems; the fourth is a downstream consequence of a
representation problem.** Implementing only the first item—removing dead variants—would leave the
synonym table and reverse dependency in place. Implementing only the fourth item—injecting a
callback into the parser—would build on an inconsistent representation.

One of the three parallel representations is an alias illusion:

| Name        | Definition location                                                       | Size        | Role                                                    |
| ----------- | ------------------------------------------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541`                                 | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`                                         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | Before 6.2.2: head of `src/middle/core/ir.rs` (the line has been removed) | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

Before 6.2.2, the head of `ir.rs` contained a `pub use … parser::ast::Type;` line followed
immediately by a `use … typecheck::MonoType;` line. The IR module therefore depended simultaneously
on two “type” symbols, even though one was merely an alias. Both lines disappeared when the alias
was removed and are consequently no longer referenced by line number.

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; the impl was deleted in 6.2.2).** `From<MonoType> for IrType`
(located at `src/middle/core/bytecode.rs:2353-2390` before the change) downgraded `MonoType` to
`ast::Type`, with **silent information loss**:

```rust
// bytecode.rs:2371-2376
MonoType::Generic { name, args } => match name.as_str() {
    "String" => IrType::String,
    "Bytes" => IrType::Bytes,
    "Tuple" => IrType::Tuple(...),
    _ => IrType::Void,          // ← any other generic type silently becomes Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

All eight `MonoType` variants collapse to `IrType::Void`, in addition to a `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; the comment explicitly describes this as a “transitional branch
before enum variants are removed”).

Here, `IrType::Void` is a **legitimate type value rather than an error signal**. Downstream code
therefore cannot distinguish “this function really returns `Void`” from “this function’s return type
was lost during bridging.” This is one reason why the “type consistency” invariant of the
first-layer validator in 07 cannot currently pass on the existing code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants targeted in 2.2—**both ends
of the lossy bridge point to dead code**.

**Current state (from 6.2.2 onward)**: the impl has been **removed in its entirety**. Decoding no
longer downgrades and reconstructs `MonoType` as `ast::Type`; all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`). Decoding now transfers the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those
eight `MonoType` variants retain their real types in signatures and type tables, and `IrType::Void`
is no longer used as a substitute for “information loss.” The `ir::Type` re-export alias (the
`pub use ... ast::Type` line at the head of `src/middle/core/ir.rs` before 6.2.2) was also reduced
to a private import—the IR layer no longer publicly re-exports a syntax-layer type. The IR layer
previously had three carriers that directly held `ast::Type` (`Instruction::Cast.target_type` /
`Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`): the first two were deleted
together with two **zero-producer** dead variants in **WBS 6.6.11** (their payloads were already
discarded downstream); the last was replaced with the IR-native narrowed carrier [`TypeDeclBody`] in
**WBS 6.6.12** (a field table `Vec<TypeDeclField>` plus the bytecode type-table entry
`self_type: MonoType`). **The IR layer no longer holds any `ast::Type`, and `src/middle/core/ir.rs`
no longer imports the syntax-layer `ast`.** The **same-named bytecode-side `BytecodeInstr::Cast` is
a separate construct** (its payload is a type-table index `u16`, not a syntax type), and is retained
unchanged.

### 2.2 Parser-Forward Zero-Construction Variants

**Definition**: the construction-point scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. **A construction point is a `Type::<variant>` path in expression position** (pattern
positions do not count), and points are **separated into three categories that must not be
conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have zero construction in the **parser-forward**
direction (reconstruction points in the other two categories are described in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction points                                                                                                                                                                                                                                                                        |
| ------------------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                       |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                       |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                       |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                       |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                       |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                       |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward point = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an explicit exemption; eliminated with 6.4 #12)                                                                                                                                                |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                       |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                           |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                           |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                           |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                           |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are match arms); **2 production AST→AST reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Definition-column line-anchor drift record (P6 6.0 measurement, 2026-10-27)**: the “Definition”
> column in this table and in the table in 5.2 was written on 2026-10-05. Seventeen lines were
> subsequently inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so
> **the definition lines of all 26 variants shifted uniformly by +17** (for example, `Int` moved
> from 432 → 449 and `NamedParen` from 533-540 → 550-557). This round **records** the drift without
> changing the table body; all subsequent references use the `ast::Type` symbol as the anchor.

The causal chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm in
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment in `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… They follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction point of `Type::Void` is an error
   fallback**—`_ => (Vec::new(), Type::Void)` at `types.rs:836`, located at the end of a match
   returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this article uses the term **“parser-forward zero-construction”**: it means neither “zero
references globally” nor “zero production construction”—11 of these variants do have production
construction points in the other two categories (reverse bridge / AST→AST reconstruction). These
tests themselves codify the behavior under test: they assert exactly what these syntactic forms
lower to as `MonoType`, even though the parser never produces them. When removing the variants,
these tests must be removed or rewritten accordingly.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The “forward zero-construction” classification above holds only in the **parser → AST
forward-construction** direction. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct seven of the variants listed
above**:

| Reverse bridge                                                   | Location                                 | Reconstructed members of the 13-variant list                                                                             |
| ---------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `From<MonoType> for IrType`                                      | `bytecode.rs:2353-2390`                  | `Int` (`2357`), `Float` (`2358`), `Bool` (`2359`), `Char` (`2360`), `Void` (`2361`), `String` (`2372`), `Bytes` (`2373`) |
| `mono_to_ast_type` (nested `fn` inside `substitute_type_in_ast`) | `middle/passes/mono/function.rs:507-529` | `Int` (`512`), `Float` (`513`), `Bool` (`514`), `Char` (`515`), `Void` (`516`), `String` (`522`), `Bytes` (`523`)        |

```rust
// middle/passes/mono/function.rs:511-527
match mono {
    MonoType::Int(n) => AstType::Int(*n),
    MonoType::Float(n) => AstType::Float(*n),
    MonoType::Bool => AstType::Bool,
    MonoType::Char => AstType::Char,
    MonoType::Void => AstType::Void,
    MonoType::TypeRef(name) => AstType::Name { name: name.clone(), span },
    // #299: Container types such as String/Bytes are now Generic and are converted back to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This scope boundary directly affects the disposition**:

- The `bytecode.rs` location disappears together with #28 in 6.3 (removing the entire
  `From<MonoType> for IrType` impl), so it adds no cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name inverse resolver (annotated `#299`) that converts `String` / `Bytes` from `Generic` back
  into dedicated variants by name. After removing `ast::Type::String` / `Bytes`, these two guard
  branches must be rewritten; after removing `Int` / `Float` / `Bool` / `Char` / `Void`, the
  behavior of the `_` arm changes (some types that previously fell into `_` silently switch to
  `AstType::Name`). This is the 47th supplementary item in 6.3.
- **Direct consequence for the gates**: a T1 gate that scans only forward construction points would
  classify the seven variants above as “having construction points,” thereby excluding them from the
  removable list. T1 must explicitly include reverse bridges in its counting scope, or its
  conclusions will contradict the disposition table in this article (see 5.7).

#### 2.2.2 Scope Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution (reconstructing an
existing AST type; **not** `MonoType → AST`). The P6 T1 gate measurement (6.0.2 baseline,
2026-10-27) found **15 variants / 28 points**, of which **5 variants / 10 points** belong to the
list of 13 above:

| Variant     | AST→AST reconstruction points |
| ----------- | ----------------------------- |
| `Union`     | `377`, `606`                  |
| `Option`    | `427`, `600`                  |
| `Result`    | `431`, `602`                  |
| `Sum`       | `467`, `632`                  |
| `AssocType` | `456`, `623`                  |

**When 6.3 removes these five variants, it must rewrite all 10 points in the same batch** (already
registered in 6.3.2 in 09); otherwise, the corresponding arms will no longer compile. **Implemented
(WBS 6.3.2, 2026-10-29)**: all 10 points were removed; see the commit message for task 6.3 for the
disposition table.

### 2.3 Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`; measured at 841-867 / **13 arms**
when this section was written, and at **839-886 / 15 arms / 16 spellings** after WBS 6.1.2) uses a
string table to map type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :843
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :850
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :851
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :863
```

**The sole reason this table exists is to bridge the AST layer’s inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but structurally
different in the AST layer (and the parser produces only the latter), `from_builtin_name` must
accept case variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"`
on the `Void` line is especially revealing—it cooperates with the `Type::Void` fallback at
`types.rs:836`.

The same knowledge of “which names are built-in type names” exists in seven copies throughout the
repository, with no consistency validation between them:

| #   | Location (**measured on 2026-10-27**; this section was written on 2026-10-05, and the original line numbers have drifted)         | Form                                                                                                                                                                                                                     |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                          |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, duplicating `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                             |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                 |
| 4   | `src/lsp/world.rs:182`                                                                                                            | List of built-in type names on the LSP side (14 names = the 15 names without `()`; WBS 6.1.2)                                                                                                                            |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` family of aliases (registered by the P0 gate on 2026-10-06)                                    |
| 6   | Base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                           | String match, name → byte size. Previously included a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports Unknown type. **Dead arm and live evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set is a name” rule (`inference/expressions.rs:2917-2918`, decision #300)                                                |

> **Line-anchor drift record (P6 6.0 measurement, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223, and the others by 2-9 lines), but **the
> number of entries and their contents have not changed** (item 1 had 13 arms when this section was
> written, item 3 had 15 entries, and item 4 had 15 entries when written; after WBS 6.1.2, item 1
> has 15 arms / 16 spellings and item 4 has 14 entries). All subsequent references use a **symbol
> anchor + checked-in path**; the line numbers in this table are provided only for location. The arm
> count of item 1 and the live drift evidence for items 5 / 6 / 7 (the dead `Uint` arm and the
> conflict between `Set` and the typecheck side) **all remain**, and this section has made no code
> changes.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to identify const generic parameters. It
enumerates “type names usable as const parameters,” which **partially overlaps but differs from**
the complete set handled by `from_builtin_name`, and no mechanism guarantees that the two remain
synchronized. Item 4 is a fourth copy used for LSP syntax highlighting and hover, likewise without
validation; T2’s comparison scope includes it.

Items 5-7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 and 7 share the same root as items
1-4 (string-keyed tables mapping built-in names to semantics / IDs), although their purposes differ
and they drift with the same risk. The same gate also found 11 string dispatches of built-in
container names on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4,
`"String" | "Bytes"` ×4, etc.). **These are outside both this section and the parser data-flow
coverage in 5.4**; the inventory and disposition are recorded in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as a P6 extended-verification item.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both of which belong to the type layer).
**Measurement differs from that description**: these two functions are defined inside the parser,
not in the type layer.

- `is_type_param_annotation` is defined in
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository’s only `fn`
  definition)
- `name_used_as_type` is defined in `declarations.rs:42-80` (the repository’s only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined in
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production-code locations** that genuinely cross layers and reference
`typecheck`. Both call the same function,
`crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                             |
| --- | --------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Identifies constraint-position parameters (`T: Add`), which do not occupy runtime parameter slots   |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Identifies constraint-position parameters and produces `GenericParamName` values with `constraints` |

The second location is deeper: **`parser/ast.rs` itself** (`ast.rs:847-948`, the entire block
containing `StmtKind` / `Expr`) directly references a typecheck symbol. Routing table C records only
the `declarations.rs` location.

Together with the hard-coding at `declarations.rs:498-499`, it forms a single judgment:

```rust
// declarations.rs:498-499
// Expanded as more built-in predicates are added (future open-set approach: pass
// predicate names down to the parser, or use a form decidable at the syntax layer).
// The parser does not hold the type environment, so it checks two sources: the hard-coded
// built-in predicates and the accumulation of declarations parsed in this pass
// (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment explicitly acknowledges this as a temporary form of the “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is a downstream consequence of the type-representation problem**: the parser must ask
whether something “is an operator-interface name” because it must distinguish whether `Type::Name`
refers to a type or an interface constraint. **`Type::Name { name: String }` uses an
undifferentiated string to carry at least four meanings** (concrete type / type variable /
constraint name / predicate name). The syntax layer cannot determine this on its own, so the
authority is forced upward into the type layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` explicitly
acknowledges this:

```rust
/// Whether the parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`'s `N`).
///
/// Semantically identical to `declarations.rs`'s `name_used_as_type`; independently implemented here to avoid
/// cross-module dependencies within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                                          | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                              | No callback                                          |
| `Type::Generic` branch  | First short-circuits through `is_predicate(app_name)` and returns `false` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls through to `_ => false`)                                           | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                           | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records the origin of the latter two branches: “These two branches
were previously missing; when a const generic parameter was referenced in a definition body, it
could not be determined that it was ‘used as a type,’ and annotation validation incorrectly reported
the legitimate const parameter as an unknown name.”

In other words, **the function was duplicated to avoid a cross-module dependency, but fixes did not
flow back to the original**. The `ast.rs` version recursively traverses fields in a definition body,
whereas the `declarations.rs` version does not—the same source fragment receives different results
at the two judgment points. This duplicate must be consolidated when `NameKind` is introduced in
5.3.

### 2.5 Parallel Operator Enums

There are two operator enums, which do not reference each other and **whose members have already
diverged**:

| Concept         | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                  |
| --------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operator | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operator  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Each also implements its own complete `Display` (`const_data.rs:291-317`) and classification
predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` in
`const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in type position, they fall into
`Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly related to the
IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears once
`Type → MonoType` is performed and is never visible to the IR, while `Assign` / `Deref` continue to
exist through paths such as `BinOp::Assign => Ok(MonoType::Void)` in `ir_gen.rs`
(`inference/expressions.rs:875`). Because the two enums have diverged, neither side can
independently determine which operators may appear in type position.

This phase does not consolidate the two operator enums (their ownership is described under “Changes
to the Operator Surface” in [05-frontend-paradigm.md](05-frontend-paradigm.md)). However, the T1
gate’s scan targets should include `ast::BinOp` / `ast::UnOp`, making “new zero-construction
operators” equally visible.

### 2.6 `Expr::FnDef`: A Production-Zero-Construction Expression Variant (Removed: WBS 6.5.1, 2026-10-30)

**This section records an implemented fact**: `Expr::FnDef` was once a variant of `ast::Expr`. The
claim that it had **zero production construction** was correct—there were only two construction
sites in the entire repository (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`), both in tests.

However, **there were far more than 4 consumption points**: the baseline measured **15 production
consumption points** (classification in 5.5), plus 6 comments mentioning the variant. At the 6.4
closeout state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1 definition + 15
production consumption points + 3 test construction points + 6 comments.

**The path actually taken by function definitions** is: `pratt/nud.rs` constructs `Expr::Lambda`,
and `parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in
`value`). In other words, the `FnDef` branch was **a parallel path that was never executed but was
maintained at 19 code locations** (15 consumption points + 3 test construction points + 1
definition).

**Implemented state**: the variant definition and all consuming arms were **removed completely**.
`git grep -n "Expr::FnDef" -- src/` within `src/` now yields 0 matches (the bare word `FnDef`
remains only in two explanatory comments). See the commit message for task 6.5 for the per-location
disposition table, test migration, and measured results; see 5.5 for the design of their
consolidation.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` once carried a typechecker-specific field (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>` (formerly
`ast.rs:244-245`), and its comment explicitly stated: “The first group of signature parameters is
retained verbatim (including parameter names) for the typechecker’s `classify_generic_params` to
use”—a statement AST reserved a dedicated slot for the type checker.

The current representation places parameter names and types **together** in the type annotation’s
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` now carries the declaration
signature as `type_annotation: Option<Type>` (the same carrier used by `Assign`).

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. It is subsequently recovered as a parameter list by the
`Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466`.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module’s match arm then restores its meaning.

**Third, `Type::NamedParen` carries the binder name in return position.** The comment at
`ast.rs:518-532` explicitly states:

> RFC-027 §3 uses it to declare the **return-position formal parameter name** (called a binder
> here)… **Without retaining it, the type checker can only guess** that “a free variable outside the
> scope of the constraint is the return-value parameter,” causing `(r: P(m))` to silently substitute
> the undeclared `m` as a binder as well (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: a single representation means `Type` (formerly `ast::Type`) as the sole type
structure; `MonoType` is demoted to the type checker’s internal working representation; and the
`ir::Type` alias is removed.**

The three representations converge to **two roles**, but only one is a “type”:

```
唯一类型结构  Type  ─────────────────────────────┐
   (26 → 13 变体, 住在 types/ 而非 parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, 无语义损失)
                                                  │  ← 唯一转换点 types/lower.rs
类型检查工作表示  MonoType  ──────────────────────┘
   (带 TypeVar / 替换态, 只在 typecheck 内部流转)
   ×
ir::Type 别名删除, BytecodeFunction.params / type_table 改用 MonoType
```

**Why not “consolidate into `MonoType`”** (this is the easiest alternative to propose and is
rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), while
   `MonoType` does not. C3 in 07 requires “identical diagnostic codes and messages”—diagnostic
   locations must remain traceable to source code. Adding `Span` to `MonoType` would make
   monomorphized types carry a mass of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   in `types/solver.rs:411`). These are **intermediate states in the type-checking solver**, not
   types themselves. Putting them in the sole type structure would require the IR construction layer
   to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Consolidating on
   `MonoType` would require adding these syntactic forms back.
4. **The decisive reverse argument**: promoting `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer’s type. RFC-039 explicitly prohibits L2 from
   depending on L3. One of this article’s goals is to eliminate reverse dependencies, so selecting a
   solution that creates one would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) caused
`BytecodeFunction.params` (`bytecode.rs:812`) and `return_type` (`bytecode.rs:814`) to use
`ir::Type`, whereas the serialization-side `codegen/bytecode.rs:277-278` used `MonoType`. The two
field types were connected through `file.type_table.into_iter().map(|t| t.into())` at
`bytecode.rs:2339` and the lossy `From` bridge at `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the two sides have different field types.”**
Once `bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly
changed to `MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed in
full, thereby eliminating the reverse reconstruction recorded in 2.2.

**Explicitly out of scope**: `Type` is not moved from `parser/ast.rs` to `sema/types/` during this
phase. This phase only adds `lower.rs` under `types/` as the sole conversion point, without changing
the definition location of `Type`. The directory rename is performed at the end of P6 under D1 as a
standalone move batch; at that point, `Type` will live in the top-level `ast/type_.rs` (the AST
top-level domain), while `sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> **Implemented (WBS 6.3, 2026-10-29)**: all 13 “delete” items in this table have **been
> implemented**—`ast::Type` was consolidated from 26 variants to 13 variants (13 deleted, 1
> migrated, 12 retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two
> AST→AST substitutions) were rewritten in the same batch. See the commit message for task 6.3 for
> measured results, the 10-point disposition table, and acceptance criteria.

> Disposition categories: **delete** (zero construction in the parser-forward direction), **retain**
> (has forward construction points), and **migrate** (semantics are reassigned to another node).

> **Line-number convention**: the “Definition line” column gives each variant’s **original location
> before removal** (the 26-variant era; 6.3 removed 13 of them); the match-point line numbers in the
> “Rationale” column use the same **pre-6.3 baseline** and are not shifted with the code. This batch
> only refreshes the definition lines for the **still-existing** variants (rows 14 and 26) and
> corrects the consumer reference that had become invalid in row 14.

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthen)                    | The parser’s **sole** primitive-type outlet (`types.rs:162-165`). After deleting the 12 primitive-type variants, all primitive types use it. A “semantic kind of name” annotation must be added (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:699` is a match arm). Bit width is carried by the `Name` path through `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372` consumes it through `IrType::String`; that arm must be rewritten after removal                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:703-706` are match arms). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The sole parser-forward construction point is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an explicit exemption). The fallback is removed with 6.4’s error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has a production construction point at `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction points (`function.rs:377` / `606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:747` is a match arm); zero across all three categories—the only globally zero-construction variant. Enums use `Struct` + `TypeBodyItem`, unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                                 | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering). (The original reference to `IrType::Fn` at `bytecode.rs:2366` disappeared with 6.2’s removal of the `ir::Type` alias and lossy bridge.) **Field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types together) is performed with 6.8.1 (D54)**—the names formerly held by `Assign.signature_params` are carried forward in this way (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous` / `from_type` / `is_named`; anonymous parameters (pure type positions) are synthesized with empty names, leaving this table’s “net effect” unchanged; `Param` has a hand-written `Debug` implementation (anonymous parameters print according to their carried type, yielding zero IR snapshot drift, see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points (`function.rs:427` / `600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that the `Generic` path is used                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points (`function.rs:431` / `602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero construction in the parser-forward direction (the syntax layer has no `::` associated-type path); **2 production AST→AST reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 unchanged**: the criterion remains forward construction; if associated-type syntax is enabled in the future, a new proposal will define the criterion at that time                                                                                                                                                                                                                                                                                                                                                                                                              |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points (`function.rs:467` / `632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has a production construction point at `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type inside unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction points at `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has a production construction point at `types.rs:160`. It is the only variant in `Type` that embeds `Expr`, and is forbidden in the IR layer (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004’s curry terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Semantics (the binder name) move to the type checker, while the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now states that “parameter names belong to `Type::Fn.params`; this node only carries the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction points must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs seven of rows 2-8 above;
> in addition, the AST→AST reconstruction points recorded in 2.2.2 contain 5 variants / 10 points
> from the list of 13 (Union/Option/Result/Sum/AssocType, two each). Both categories of
> reconstruction points must be handled in the same batch as 6.3.1; otherwise, behavior either
> changes silently (see 6.3 #47) or compilation fails outright.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly according to
> D8 (row 18 of the 5.2 table). The **criterion is based on forward construction**: forward zero
> construction holds (the syntax layer has no path that produces an associated type with `::`), so
> D8 does not need to be changed and the variant does not enter the allowlist. **The 2 AST→AST
> production reconstruction points** (`function.rs:456`, `substitute_type_ast`, and
> `function.rs:623`, `substitute_type_in_ast`) were registered in 6.3.2 and are rewritten in the
> same batch as 6.3. The three-category split in the T1 gate (5.7) reproduced this conclusion (the
> 6.0.1 commit measured 26/26 variants in agreement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine whether something is a
“predicate,” the same file at `510-512` must determine whether it is an “operator interface,”
`src/frontend/core/parser/ast.rs:971` must determine whether it is an “operator interface,” and the
two implementations of `name_used_as_type*`—`name_used_as_type_in` in the same file (measured
at 864) and `name_used_as_type` in `declarations.rs:42`—must determine whether it is a “type
reference.” All of this relies on string comparisons.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target shape (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // Added
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → passed to from_builtin_name
    UserType,     // User-declared type name (including Generic std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: classifying `NameKind` requires exactly the information the parser cannot
currently obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). This field therefore **cannot be classified independently inside the parser**—it
must be classified by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (implemented in WBS 6.1.2, finalized by the user on
2026-10-27)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:1121-1143`) is **retained, with
reduced semantics**. Before the change, it was a disambiguator that bridged two equivalent AST
representations; after `Type::Int(64)` was removed, “Int” had only one representation path, and the
table degenerated into the **sole name-to-type parsing table**.

It must not be removed for the following reasons:

- It is the **sole cut point** for standard-library type names and language built-in names (the
  signature strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP list all refer to it).
- The `DateTime` arm records a real fix (issue #338 collateral: the return value of `now()` could
  not be passed to `format_time`)—it is a **semantic alias**, not the same kind of alias as the
  `Int` / `int` case variants, and is therefore retained separately until P10 S11 (issue #513).

**Disposition result** (final criterion = 15 built-in names; the temporary `DateTime` alias was
removed with S11.1 (issue #513)—the bare name now reports E1003/E1002):

| Category                                                                                                                                                                                                                                     | Before                                           | Disposition                                                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Mixed into 13 arms with aliases                  | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                          |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                         | **Deleted** (22 spellings); **no lexer alias table is introduced**—the corpus contains 0 noncanonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | In the same arm as `Void`                        | **Retained**—`()` is a spelling of `Void` (standard-library signature strings write `() -> Void`), not another type                                                                                            |
| `DateTime`                                                                                                                                                                                                                                   | `datetime` and lowercase aliases in the same arm | Removed with S11.1 (issue #513—the alias arm was deleted, and the bare name reports E1003/E1002)                                                                                                               |

**Net effect**: `from_builtin_name` = **14 match arms / 15 spellings / no aliases** (`()` and `Void`
share one arm; the temporary `DateTime` alias was removed with S11.1 (issue #513)). After deleting
the 22 spellings, noncanonical spellings in type position become `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—**they are no longer silently
accepted**. Previously, `add_builtin_types` also registered seven lowercase duplicates (`int` /
`float` / `bool` / `string` / `void` / `never` / `char`) in the type table, constituting a second
alias table; these were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names; the negative case already asserts
   `from_builtin_name("DateTime") == None`—`DateTime` no longer maps to `Int(64)`);
2. **T2.5, the “deleted-spelling sentinel,”** in `tools/type-tables --report`—if any of the 23
   entries (the 22 spellings deleted by the 6.1.2 final decision plus `DateTime`, removed by S11.1)
   is added back to the parsing table, the report immediately counts it as **unregistered drift**
   (there is no disposition that can be registered: that would be a rollback).

**Consolidating the two `name_used_as_type*` implementations**: the two duplicate implementations
recorded in 2.4 must be consolidated into one, or the classification information supplied by
`NameKind` will be bypassed by two independent string-comparison implementations. The consolidated
implementation belongs in `parser/ast.rs` (next to the data it operates on), and `declarations.rs`
should call it; the `is_predicate` short-circuit should read `NameKind::Predicate`, and the `Struct`
/ `NamedStruct` field recursion should follow the more complete implementation at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: the const-generic-parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps the complete set handled by
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the check in
`extract_generic_param_names` (same file, measured 932) can be changed to “`kind == Builtin` and the
name is in the accepted const-parameter set,” explicitly connecting the two sources of knowledge;
the subset definition for const parameters is validated by the T2 gate (see 5.7).

> **Design judgment**: if a `.yx` corpus contains type annotations such as `i64` / `int`,
> normalizing them to `Int` changes the **diagnostic locations** of those files (but not their
> diagnostic codes). C3 permits this (messages and ordering may be normalized), but the baseline
> must be recorded in stage 1. **Measured (P6 6.0, 2026-10-27)**: a full scan of `tests/yaoxiang`
> (316 `.yx` files), `tests/yaoxiang-multifile`, and `src/std` found **0 noncanonical names in
> actual type positions**; the 16 raw token matches were adjudicated individually as 12 strings and
> 4 comments (both `i64` / `f64` occurrences were in comments). Consequently, 6.1’s name
> normalization **does not change diagnostic locations in any corpus**, and **there is no need to
> regenerate the baseline**.
>
> **Retest (P6 6.1 closeout, 2026-10-28)**: 6.1.1 rescanned the **22 spellings** finalized on
> 2026-10-27—the 2026-10-27 measurement used a **narrow scope** (only counting the 14
> integer/floating-point-family spellings and excluding namespace usage in `src/std`, hence the
> earlier record of “16 raw-token matches = 12 strings + 4 comments”). The retest scope was 316
> `.yx` files in `tests/yaoxiang` + 39 in `tests/yaoxiang-multifile` + 38 in `src/std` = **393
> files**, with **308 matching lines**; only **2 lines were type-position candidates, both inside
> `//` comments** (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`, `i64::MAX`, and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`, `-> f64`). Therefore, there are still
> **0 actual type positions**; the remaining matches were adjudicated individually as string
> literals / comments / the `std.string` module name and namespace access / a `Json` variant name /
> a `void` literal, and **none is a type annotation**. See the 6.1.1 row in
> [09](09-execution-wbs.md) for the retest command. The conclusion is consistent with 6.0: 6.1’s
> name normalization **does not change diagnostic locations in any corpus**, and **there is no need
> to regenerate the baseline**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed loop inside the parser + 2 out-of-bounds accesses):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injected, without crossing boundaries):

```
L1 编排层（入口处一次性构造）
   │  ① 编译内置运算符接口表
   │  ② 编译 std 的 .yx 声明 → 谓词名集合
   ▼
TypeEnvProbe（trait，定义在 L2，src/frontend/core/parser/probe.rs）
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState（parser_state.rs:46/54 处扩展，改为委派给 probe）
   ▼
declarations.rs:498-511 与 ast.rs:930 改为
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Per-file changes**:

| File                                                                                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                     | Define `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is valid dependency inversion. **The implemented method names are authoritative** (the original design’s `is_type_var` / `is_builtin_type` were not adopted: a type variable’s identity is determined structurally by its declaration site, not by its name, so no such criterion is defined; built-in-name classification is instead provided by `kind_of`, and `knows_builtin_types()` declares whether this probe holds the built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                              | Retain `declare_predicate` / `is_predicate_name` (for accumulation within the current pass); add a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**a borrow rather than a `Box`**). **The implemented state also adds a `kind_of()` accessor**—predicates declared in the current pass take precedence, and all others are delegated to the probe; this is the sole entry point for name classification                                                                                                                                                                                                                                  |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                 | Remove `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                 | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/parser/ast.rs:930`                                                                         | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and should not hold an environment dependency—the function must accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                                    |
| All **40** parser construction sites (`ParserState::new` is the sole constructor; **4 production + 36 test**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. The **default implementation, `NullProbe` (all results false)**, ensures that parser unit tests remain unaffected; the default production-entry probe is `BuiltinProbe` (the compiler’s built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (The original estimate of “5 construction sites” came from preliminary reconnaissance; measurement found **40**—`Parser::new` does not exist, and `ParserState::new` is the only constructor)                                                                                                                   |

**The `extract_generic_param_names` signature problem**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that accepts no context. After removing the out-of-bounds call at `ast.rs:930`, it
needs an `is_constraint` check, so its signature must add `probe: &dyn TypeEnvProbe`. Callers
(including `parser/tests/ast.rs`) must pass `&NullProbe` accordingly.

**When `NameKind` is classified**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to classify `kind`, fixing it once so that later match arms no longer perform string
comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them concerned parser-internal
functions under the scope in 2.4, while the genuine cross-layer references are the 2 locations
covered by this section, `declarations.rs:509` and `ast.rs:930`.

### 5.5 Consolidation of `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Proposition: remove `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This path was already **the operating path** (`pratt/nud.rs` constructs
`Expr::Lambda`, and `parser/statements/declarations.rs` constructs `StmtKind::Assign`; see 2.6); the
`FnDef` branch was a parallel implementation that was never executed.

**Implemented state**: the variant definition and all **15** production consumer arms were **removed
in the same batch** (Rust’s exhaustive `match` makes “definition + additional arms” an atomic unit;
see “Registration ①” in the commit message for task 6.5). Baseline consumer points are classified by
form as follows:

| Category                                                                       | Count | Disposition                                                                                            |
| ------------------------------------------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------ |
| Variant definition                                                             | 1     | Delete                                                                                                 |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (otherwise compilation fails)                                                                   |
| Non-exhaustive match arms + or-pattern with `_ => Span::dummy()`               | 10    | Delete (after removal they fall through to the fallback; retaining them would create a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after removal they never match, with no error or warning)                                      |
| Test construction points                                                       | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not remove the test cases)                                    |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
matching the first three fields of `FnDef`; the `name` field unique to `FnDef` has no usable
location at the `Expr` layer (the name resides on `StmtKind::Assign.target`).

**Risks and mitigations (originally planned, now implemented)**: the 2 non-exhaustive `if let` sites
become impossible to match after the change—no error and no warning. Mitigation is **guaranteed at
compile time**: after the variant is removed, any new `if let Expr::FnDef` fails to compile because
the variant no longer exists, and **no gate is required**.

**Per-location disposition table, test migration, and measured results**: see the commit message for
task 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**Remove `signature_params: Vec<Param>` from `StmtKind::Assign` / `StmtKind::TypeDefinition`.**

Rationale: its reason for existing (the original comment at `ast.rs:244`) was “for the typechecker’s
`classify_generic_params` to use”—a **dedicated slot reserved by statement AST for the type
checker**, duplicating the transport of another representation of the same parameters.

**The implementation scope differs from the initial draft of this section in one critical respect**
(corrected after stage-1 verification): the initial draft proposed changing the data source to
“recompute `Lambda.params` in place,” but `Lambda.params` contains **already-filtered** value
parameters (type-position and constraint-position parameters removed), whereas
`classify_generic_params` / `extract_generic_param_names` require the **unfiltered** original
signature list. The actual implementation therefore first moves the names into the sole carrier,
then removes the old field:

1. 6.8.1 first reshapes `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   together, reusing the existing `Param`—no second structure and no parallel arrays);
2. 6.5.2 then removes the two `signature_params` fields: `Assign` obtains them from
   `type_annotation`, while `TypeDefinition` now carries the declaration signature as
   `type_annotation: Option<Type>` (the same carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (curry groups are
   expanded in order; `Paren` / `NamedParen` are transparent). The `NamedParen` binder is not
   included; the type checker interprets it explicitly (see below).

> **Risk-verification conclusion**: the initial draft’s concern about the difference between
> “unfiltered” and “filtered” was valid—but **it is not a problem**: the flattened result and the
> old `signature_params` are element-for-element derived from the same `Param` nodes, so generic
> classification does not change. The corpus result of 356/356, the 216 zero-drift IR snapshots, and
> a fully green `cargo test --all` provide the evidence (see the commit message for task 6.5). The
> initial draft’s “use `Lambda.params`” approach would have changed generic-classification results
> and **was not adopted**.

**Retain `Type::NamedParen` (`ast.rs:533-540`) as a node while moving binder semantics out of it
(completed: 6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records its **sufficient reason for existing** (see 2.7): in
`(r: P(m))`, dropping `r` would leave the type checker guessing that “a free variable outside the
scope of the constraint is the return-value parameter,” which would **silently substitute the
undeclared `m` as a binder as well** (the comment marks this as a “measured defect”).

It therefore **cannot simply be removed**, because doing so would reintroduce a correctness defect
that has already been fixed. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param` /
   `param_span` carry source facts only and do not carry type-layer binder semantics.
2. Move binder interpretation authority to the **type-checking layer**: when consuming `NamedParen`,
   the type checker **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node’s
   field,” the same category as `Expr` embedded in `ConstExpr`: the syntax node supplies a fact, and
   the semantic layer interprets it.
3. Rewrite the comment from “the type checker can only guess” to “parameter-position formal names
   belong to `Type::Fn.params`; this node only carries the **return-position** binder.”

> **Path correction**: `sema/` in the initial draft of this section is the **target** directory name
> from RFC-039 decision D1; the semantic-layer directory in the current repository is
> `src/frontend/core/typecheck/` (the rename belongs to batch 6.7 and has not yet been performed).
> All code paths in the body of this article are written against the pre-rename state (see
> “Directory Renaming and the Path of This Article”).

**Why this is a “migration,” not a “deletion”**: the syntax tree must represent source code without
loss. `r` genuinely exists in the source, so the AST cannot pretend otherwise. The problem is not
the node’s existence, but the failure to define explicitly which layer is responsible for
interpreting its semantics.

### 5.7 Build-Time Type-Table Gates

**The reference implementation has already been validated in this repository**: `build.rs:19-37`
calls `parse_registry` / `validate` in `tools/code-tables`, performing uniqueness, segment, and
Chinese-completeness checks over 145 error codes; when `is_ok()` is false, it directly executes
`panic!` to reject the build. `build.rs:39-54` further compares the RFC-013 Markdown code table
entry by entry and likewise executes `panic!` on a mismatch. **RFC-013 documentation and code
therefore remain consistent at all times**—`tools/code-tables/Cargo.toml:15-16` has only one
dependency, `serde_json`, and the validation logic performs text-level parsing without relying on
syn.

The type tables currently have **no** comparable gate. The mechanism added by this article can
directly reuse the established form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). The checks are as follows (the T3 column was removed on 2026-10-30; the deletion
record is retained):

| #                                                              | Check                                                                                                                                                                                                                                                                                                                                                                                                                       | Input                                                               | Failure condition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Remedy                                                                                                                                                                                                                                                                                                                                                                                               |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **T1 construction-point split**                                | Scan the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant’s construction points in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`). Reconstruction locations (②③) are driven by the `REBUILD_FUNCTIONS` table in `scan.rs` and reconciled line by line against source code | Variant list + three-category match table + reconnaissance baseline | ① A variant has **zero forward construction** and is not registered; ② a registered variant has acquired a forward construction point; ③ a registered wildcard-arm fallback exemption no longer matches; ④ an **unregistered wildcard-arm fallback point** exists (fallback exemptions are registered individually and must not be generalized to every wildcard arm); ⑤ **reconnaissance baseline (T1.6) drift**: a variant’s total/forward/AST→AST/reverse-bridge counts differ from `SCOUT_BASELINE` (decision 2A); ⑥ the reconstruction-location registry differs from source code: a registered reconstruction function has no same-named `fn` in source code (renamed/deleted without synchronization); ⑦ **criterion and baseline are from different generations**: the current `REBUILD_FUNCTIONS` differs from the frozen criterion snapshot `SCOUT_BASELINE_CALIBER` (criterion revision) (6.6.3) | Delete the variant; or register a rationale / remove an invalid registration (see 5.2 and `REGISTERED_ZERO_FORWARD`); refresh `SCOUT_BASELINE` with reproducible evidence if the baseline drifts (including construction-point file:line); changing the criterion requires refreshing the criterion snapshot and baseline numbers in the same batch; keep the registry synchronized with source code |
| **T2 synonym-table and repository-wide name-list consistency** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `sema/probe.rs` (moved back from `parser/ast.rs` to L3 by WBS 6.6.9) + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                                                                                                            | Three tables                                                        | A type name exists in one table but is not recognized by another (compared according to each table’s semantic mapping rules); **since WBS 6.1.2, T2.5 “deleted-spelling sentinel” also exists: adding back any of the 22 deleted spellings reports red**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | `type-tables --fix` fills the gaps bidirectionally                                                                                                                                                                                                                                                                                                                                                   |
| ~~**T3 built-in type names match RFC-011 documentation**~~     | Originally: the canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                                                                                                                            | —                                                                   | **Removed following the user decision on 2026-10-30 (the comparison target does not exist)**: RFC-011 is a generic type-system design and its body has no table with “built-in type names” as a column                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | — (not applicable; see the explanation after the table for the deletion rationale and coverage)                                                                                                                                                                                                                                                                                                      |
| **T4 `Type → MonoType` exhaustiveness**                        | The `Type` variant list vs the match arms in `types/lower.rs`                                                                                                                                                                                                                                                                                                                                                               | Two lists                                                           | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Add an arm / remove an arm                                                                                                                                                                                                                                                                                                                                                                           |

> **T3 deletion and T2 coverage (user decision, 2026-10-30)**: T3’s **comparison target does not
> exist**—RFC-011 is a generic type-system design, and its body contains no table with “built-in
> type names” as a column. The initial implementation had to treat “every token wrapped in inline
> code in a Markdown table row whose text is an identifier beginning with an uppercase letter” as
> the documentation-side criterion, which necessarily produced an empty shell of “no disposition
> required” differences (16 registrations: 3 documentation-only + 13 code-only, none of which could
> be adjudicated as true or false). Therefore, the check, its 16 registrations, and the
> corresponding `--report` / `--json` output sections were deleted in the same batch, leaving no
> placeholder on the code side.
>
> **The only carriers of built-in type names are the three code-side lists, all of which are now
> reconciled name by name by T2**: ① `from_builtin_name` (`src/frontend/core/types/mono.rs`;
> accepted surface = 15 canonical names, no aliases—the temporary `DateTime` compatibility alias was
> removed with S11.1 (issue #513), and the `--report` T2.1 measurement is “14 arms / 15 spellings,”
> with the first spelling of each arm being canonical; the 22 implementation spellings deleted by
> 6.1.2 and the `DateTime` removed by S11.1 are guarded by the 23-entry T2.5 sentinel); ②
> `CONST_PARAM_TYPES` (`src/frontend/core/sema/probe.rs:57`—moved to L3 by WBS 6.6.9; **a subset of
> ①**, check C1); ③ the LSP built-in type-name list (`src/lsp/world.rs`; **a superset of ①**, checks
> C2 / C3). No equivalent list exists on the documentation side: the authoritative location for a
> new built-in name is ①, and T2 reports gaps in ②③ as **unregistered drift**.

**T4 implementation note**: Rust’s exhaustive match already guarantees the “variant → arm” direction
at compile time. T4’s value lies in the **reverse** direction—detecting a stale arm in `lower.rs`
that points to a deleted variant. Such code would already fail to compile when the variant was
removed, making T4 redundant in practice. **T4 is therefore demoted to a CI assertion**
(`cargo build` success is sufficient) and is not placed on the `build.rs` `panic!` path. **T1 and T2
are the two checks that genuinely require text parsing** (T3 has been removed, as described above).

**T1 failure form (6.6.2, user decision 2A, 2026-10-30)**: T1 has changed from “report only” to a
**hard build failure**—`build.rs` calls `type_tables::build_gate(root)`: `Ok(summary)` prints one
`cargo:warning` line (an informational line, not a violation), while `Err(details)` directly
executes `panic!` to reject the build. **All six categories of violation use the same failure path**
(see ①–⑤ and ⑦ in the table above; ⑥ belongs to the scanner-layer failure path
“reconstruction-location registry differs from source code” and, like the different-generation
criterion added in 6.6.3, fails directly, but is not counted as a gate-violation category). Each
error includes the **variant name** and the `file:line` of the construction point / fallback point /
variant definition; baseline drift also includes location information (since 6.6.3). Failure to
complete the scan itself (file-parse failure or incomplete criterion) also counts as failure and
**must not be downgraded to “pass.”** The registration mechanism is retained with empty tables:
`REGISTERED_ZERO_FORWARD` and `DEAD_VARIANTS_03` are currently empty (after 6.3 removed the 13 dead
variants, no zero-construction variants remain). T1.7’s “registration no longer valid / exemption no
longer matches” alarms and injected verification (`*_with(registry)`) continue to work—any newly
introduced zero-construction variant must be deleted or have its rationale registered, or the build
fails.

**Single source of truth for reconstruction locations (6.6.2, user decision 1A)**: classification
into the second category / reverse bridge is driven by the `REBUILD_FUNCTIONS` table in `scan.rs`
(**file + function name → category**). `Type::without_fn_param_names` (the fidelity-preserving
reconstruction introduced in batch 6.5: it recreates an existing AST type to remove type-position
parameter names; its sole consumer normalizes the type before entering the IR in `ir_gen.rs`;
**since 6.6.12 it was removed together with the narrowing of the IR carrier**, and its registration
and criterion snapshot were removed in the same batch) belongs to the **second category, AST→AST
reconstruction**. Previously it was counted as parser-forward, artificially adding one point to the
forward count of 11 variants (`Name` / `ConstExpr` use `self.clone()` and do not produce
construction points). The registry is reconciled **line by line against source code**: if a
registered function has no same-named `fn` in source code, the scan reports an error and **must not
silently fall back to the forward category** (otherwise, after a rename or deletion, a
reconstruction point could masquerade as a construction point).

**The baseline numbers pin down two things** (6.6.3 review F1/F2):

1. **The source tree**—`SCOUT_BASELINE_REV`; point counts are meaningful only for the same source
   tree;
2. **The counting criterion**—`SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` (a frozen
   snapshot of `scan.rs::REBUILD_FUNCTIONS` at that revision, keyed by “file + function →
   category”). The snapshot is **not stored in the same table as the registry**, specifically so
   that a criterion change becomes a detectable event: storing them together would allow both to
   change simultaneously, making the change permanently invisible to the gate.

The gate performs a machine-based reconciliation of the snapshot and current registry
(`baseline::caliber_drift`, modeled on `scan::missing_rebuild_functions`): adding/removing a
reconstruction function, or changing a function’s category, reports **“criterion and baseline are
from different generations”** (including the criterion revision and itemized differences), and
**skips** baseline-drift comparison—point counts across criteria are not comparable, and continuing
the comparison would disguise a criterion change as numerical drift. The T1.6 section of the
`--report` output prints the baseline revision and **criterion revision**, allowing read-only report
users or script runners to reproduce both calculations.

**Two implementation difficulties in T1 and their mitigations**:

1. **Distinguishing construction points from match arms** requires syntactic analysis. The
   repository-root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has
   only `serde_json`. Mitigation:
   - **Preferred**: introduce `syn` into `tools/type-tables` only (without modifying the root
     crate). `syn` is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing line-level heuristic in `tools/extract_arm.py` (an existing
     Python tool in the repository)—construction points occur on the right side of `= ` or in
     function-argument position, whereas match arms occur on the left side of `=>`. Use the number
     of occurrences of `=>` as an approximation.
   - **Conservative starting point (completed in this order)**: when T1 was first introduced, it
     **reported without failing** (`cargo:warning`) so that a full baseline could be established on
     **unmodified** code and the heuristic verified. Since 6.0.1 it has reported only; **since 6.6.2
     (2026-10-30), it has used `panic!`**, and by the time it became a hard gate, the baseline had
     already been refreshed under the same criterion to 13/13 matching rows. This is consistent with
     the approach of “the CI gate will initially be red and needs a baseline established first.”
2. **Reverse bridges and AST→AST reconstruction points must be classified separately**. Section 2.2
   records the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; 2.2.2 records the AST→AST reconstruction points in the two `function.rs`
   substitution functions (15 variants / 28 points, including 5 variants / 10 points from the list
   of 13). If T1 conflates them with forward construction, `Int` / `Float` / `Bool` / `Char` /
   `Void` / `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` would be
   classified as “having construction points” and exempted from deletion, directly contradicting the
   disposition table in 5.2. The T1 report must separate the three categories of construction
   points, and **fallback exemptions apply only to individually registered entries** (currently only
   the parser error fallback for `Void` at `types.rs:836`), not to every wildcard arm.

**Gate coverage (implemented in 6.6.2, user decision 2A, 2026-10-30)**: `build.rs` calls
`type_tables::build_gate(root)` immediately after the `code-tables` validation block, using the same
`panic!`-based build-rejection model—`Ok(summary)` prints one `cargo:warning`, while `Err(details)`
executes `panic!("Type-table gate T1 failed (build rejected): …")`. **All six categories of
violation use the same failure path**: an unregistered zero-construction variant / a registration
whose variant has acquired a forward construction point / a registration exemption that no longer
matches / an unregistered wildcard-arm fallback point / **criterion and baseline are from different
generations** / **reconnaissance baseline (T1.6) drift**. Errors include the variant name and the
construction point, fallback point, or variant definition `file:line` (drift entries have included
construction-point locations since 6.6.3). Failure to complete the scan itself (such as a file-parse
failure) is also a failure and **must not be downgraded to “pass.”**

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                                                                                   |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` in `types/substitute.rs:112`, and `substitute` in `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                                     |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `unify` in `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                                                                                            |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. Eight locations in `mono/function.rs` (`358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`) match `AstType::NamedStruct` / `AstType::AssocType`. The disposition of `AssocType` is deletion, and all 8 locations were synchronized. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3; it now uses `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match was consolidated with 6.3; the former direct read of `signature_params` in `let params: Vec<MonoType>` was replaced with flattening through `Type::fn_params_flat` (the verification conclusion in 5.6: the unfiltered list, derived from the same source as the old field)                                                                       |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Requires changes**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                                                                                                               |
| Interpreter (`backends/interpreter/`)                        | **Requires changes**. `type_table: Vec<ir::Type>` at `image.rs:44` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects the `{:?}` output format                                                                                                                                                                                         |
| RFC-027 compile-time types                                   | **Requires changes**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                                                                                                                     |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by the “type consistency” invariant of the first-layer validator in 07.
`ast::BinOp::Assign` / `ast::UnOp::Deref` (2.5) are expression-side forms of the same problem—they
may occur inside `ConstExpr`, and therefore must not descend into IR type annotations either.

### 6.2 Runtime Behavior

**This article does not change any runtime behavior.** The basis is:

1. The 13 deleted variants had no construction points in the **parser-forward** direction (the one
   forward point for `Void` was an error fallback—**WBS 6.3 changed that fallback to
   `Type::Name{"()"}` and removed the gate-exemption registration**, so no forward construction
   points remain)→ the parser does not produce them. The reverse-bridge and AST→AST reconstruction
   dispositions are described in #28 / #47 in 6.3 and the 10 points in 6.3.2, all of which have been
   implemented.
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction in which behavior changes, and it is a fix rather than a
   regression**, but it must be confirmed as regression-free by the third-layer corpus differential
   in 07.

**Semantic change points requiring particular attention**:

| Location                | Current state                                                                                 | After convergence                               |
| ----------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retains the real `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retains the real `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Deleted** (an exhaustive match is sufficient) |

> **Design judgment**: the fact that all 8 variants currently collapse to `Void` means that **every
> code path that depends on signature bytecode (the interpreter’s arity/type checks and the
> serialization format) currently sees `Void`**. After convergence, these locations will see the
> real types. **This is the only possible source of behavioral change in this article, and its
> direction is “restoring correct information.”** If the corpus differential produces a difference,
> it should be treated as **exposing an existing defect**, not as a regression introduced by this
> article. The method for determining this is to construct a `.yx` program containing a `Ref`
> parameter and compare `dump_bytecode` output before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                        | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Category                          |
| --- | ---------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`      | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**completed: WBS 6.3.1, 2026-10-29**—all 13 variants deleted, consolidating `ast::Type` to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                   | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original location, **deleted**)    | Delete `Expr::FnDef` (**completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consumer arms were removed in the same batch, leaving zero references in `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                   | Delete `Assign.signature_params` (**completed: WBS 6.5.2, 2026-10-30**—the same-named field was deleted from `TypeDefinition` in the same batch, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm location, **deleted**) | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #3 (completed)            |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                   | Change the `NamedParen` comment to “syntax supplies the binder name; semantic interpretation occurs in the **type-checking layer**” (**completed: WBS 6.5.2, 2026-10-30**—the comment was changed to “parameter names belong to `Type::Fn.params`; this node only carries the return-position binder”; `sema/` in the initial draft was the D1 target directory name, whereas the current repository uses `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                             | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Addition                          |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                         | `trait TypeEnvProbe` + `NullProbe` (**completed: WBS 6.4.1, 2026-10-29**—the implemented state also includes `BuiltinProbe` and `NameKind`; trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Addition                          |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                     | Add the `probe` field and accessors (**completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, and the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites are connected**: 4 production + 36 test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                   | Use `state.probe()` to determine `NameKind` (**not done**: 6.4 introduced only the **classification enum + probe queries**, without changing the structure of `Type::Name`—fieldization would affect every production and test construction site and require the reverse bridge `mono_to_ast_type` to preserve `kind` when writing back, requiring regeneration of the IR snapshots and corpus baselines and conflicting with this batch’s C3 surface (zero drift); reevaluate `NameKind` fieldization when a second consumer appears)                                                                                                                                                                                                                                                                                                                                                                                         | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                   | Update the comment (`Result`/`Option` use `Generic`, already consistent with the facts; only add an explanation of the variant deletion)—**not done**: 6.4 did not modify `types.rs` (the change surface of this file was the fallback refactoring in 6.3), so this line remains deferred with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                       | Delete the `_ => (Vec::new(), Type::Void)` fallback and replace it with an explicit error return—**WBS 6.3.1 changed the fallback to a let-else explicit return of `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical). **The “replace with an explicit error return” half was not done**—the original plan deferred it to 6.4’s error-path refactoring, but **6.4 did not take over that path** (6.4 only introduced injection for type-name dependencies and did not modify `types.rs`)→ **6.5 did not take over it either** (that batch only reshaped representations and left the fallback path unchanged)→ **it moved to 6.6**, where it was registered in the 6.6.1 row of 09. The path is syntactically reachable (the `Name` arm of `semantic_tokens` records one additional dummy-span token; the corpus contains 0 matches, so there is zero drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                   | Delete `n == "Terminates"`; use `state.probe().is_predicate(n)` (**completed: WBS 6.4.3, 2026-10-29**—the implemented state reads `NameKind::Predicate` through `ParserState::kind_of`: predicates declared in the current pass take precedence, and all others are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                   | Delete the `typecheck::operator_interfaces::spec()` call (**completed: WBS 6.4.3, 2026-10-29**—replaced with `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                       | Delete the `signature_params` argument—**completed (WBS 6.5.2, 2026-10-30)**: all construction sites deleted the field; in the same batch, the name-merge segment’s HashMap name table was changed to positional alignment (justified by the precondition that prior validation guarantees identical names at every position)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                        | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Follows #3 (completed)            |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                       | Retain (the parameter-list recovery point)—**the change “read Lambda directly,” proposed in 5.5, was not made**: it was not included in the 6.5.1 change list, and its disposition is to be decided together with the declaration-driven checking mode in 6.8.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`      | Delete 13 match arms (one for each deleted variant) (**completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                   | Remove synonyms (case aliases, `i64`-style abbreviations, `"()"`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Convergence                       |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                       | Retain the `DateTime` alias (**completed: S11.1 (issue #513)**—the alias was removed, and the bare name reports E1003/E1002)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                         | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Addition                          |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                               | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Convergence                       |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`)   | Delete `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                       | Keep `FunctionCode.params` as `Vec<MonoType>` (already the case)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Convergence                       |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                       | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Convergence                       |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                      | Change the `type_table` conversion to `.collect()` (the elements are now the same type)—the actual change is to **delete the conversion and transfer the values unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)          | **Delete** the entire `From<MonoType> for IrType` impl (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                        | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Convergence                       |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)                   | Handle the `AstType::AssocType` arms according to the gate conclusion—**the conclusion was determined and implemented: WBS 6.3.2 removed all 5 variants / 10 points registered in 03 §2.2.2** (`Union`/`Option`/`Result`/`Sum`/`AssocType`, two each), and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                | Type-table gates T1/T2 (the original T3 was removed following the user decision on 2026-10-30)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                       | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                       | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                      | Same; obtain the name from `Assign.target` instead                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                              | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                       | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                               | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                   | Delete the `Expr::FnDef` test (**completed: WBS 6.5.1, 2026-10-30**—the test was not removed; it was rewritten as two Lambda-destructuring cases plus one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                   | Rewrite as Lambda + Assign (**completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`             | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                   | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches in `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be handled in the same batch as #1**—**the criterion was corrected (completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`); after 6.1.2 deletes lowercase spellings, it would become `TypeRef` and **silently lose type identity**. The actual implementation therefore uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                                                                                                     | Convergence                       |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` (**completed: WBS 6.4.3, 2026-10-29**—the implemented signature is at `ast.rs:927`; the 3 production callers in `ir_gen.rs` pass `SemanticProbe`, and the others pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                     | Delete this file’s `name_used_as_type` and call the consolidated implementation in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate` (**completed: WBS 6.4.4, 2026-10-29**—the consolidated implementation is at `ast.rs:848` and covers header detection + predicate short-circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                       | Connect the `CONST_PARAM_TYPES` check through `NameKind::Builtin` (see 5.3) (**completed: WBS 6.4.5, 2026-10-29**—dual-knowledge connection: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe that does not hold the built-in-name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Convergence                       |

> **The 6 lines implemented in 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed on
> 2026-10-28) implemented these 6 lines together—four carriers changed to carry `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType`
> and the `ir::Type` re-export alias were removed, and decoding was changed to transfer values
> unchanged (`bytecode.rs:2315`, `:2316`, `:2346`). **The line numbers of these items have been
> refreshed to the implemented state** (the original anchors `812` / `814` / `854` / `2339` /
> `2352-2390` were pre-change locations). See the commit message for task 6.2 for measured results
> and gate outcomes.

> **The gates themselves must also be gated**: `tools/type-tables`, like `code-tables`, requires
> `mod` declaration wiring (the repository has already learned from 8 orphan test trees). Register
> the new crate in the root `Cargo.toml` when it is created.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the 293 `.yx` corpus files in `tests/yaoxiang/` (single-file), the multi-file corpus
layer established in P2 (`tests/yaoxiang-multifile/`), `src/std/tests/*.yx`, and the standard
library **must all retain identical behavior** (identical diagnostic codes and messages; ordering
may be normalized).

| Risk                                                                | Assessment                                                                                                                                                                                                                                                                                                               | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains type annotations such as `i64` / `int` / `f64` | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                        | **Measured** (end of 5.3): 0 noncanonical names in actual type positions → 6.1.2 directly deleted the 22 spellings and **introduced no lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Source code contains `DateTime`                                     | The temporary compatibility alias was removed from the built-in-name table with S11.1 (issue #513) (the alias arm was deleted, and parsing now begins at `mono.rs:1123`, with 15 names and no aliases); it is not a built-in name                                                                                        | The compatibility window is closed: before removal, `DateTime` was synonymous with `Int`, with no behavior change; after removal, a bare `DateTime` annotation becomes `TypeRef` and reports E1003/E1002 (a category-D semantic change approved by the user)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `("()")` occurs in type position                                    | It is not a product of the `Name` form but the spelling of `Void` (standard-library signature strings write `() -> Void`)                                                                                                                                                                                                | **Retained**: `()` and `Void` share an arm and are among the finalized 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `dump_bytecode` output changes                                      | The element type of `type_table` changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                              | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only in C1/C2 stages). The output diff must still be recorded in stage 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Behavior changes after the lossy bridge is fixed                    | See 6.2                                                                                                                                                                                                                                                                                                                  | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `mono_to_ast_type` rewrite changes type-substitution results        | `function.rs:507-529` is the output path for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting the variants, taking the `_` arm would change the result from `AstType::Int(64)` to a form such as `AstType::Name { name: "i64" }` | Stage 3 targeted comparison: select a corpus containing generic substitution and compare the monomorphized artifacts’ `type_name()` before and after the change. **Do not assume that `{:?}` output is equivalent.** **The criterion was corrected (stage 3 measurement)**: equality must not be judged using `type_name()`—it is the lowercase internal name used for diagnostic display, so comparing it can reveal only that both values are “`int64`,” not whether the canonical name is `Int64` or `i64`. Instead, judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()`, mutually inverse for every name) + structural comparison of the written-back artifacts. WBS 6.3 measurement: the dumps and `.42` artifact md5 values of 4 generic-substitution programs were byte-for-byte identical to their pre-change values |
| `.yx` source itself                                                 | **Zero modifications**                                                                                                                                                                                                                                                                                                   | This article does not require changes to any corpus or standard-library source file. This is a hard acceptance requirement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

**Hard acceptance condition** (**criterion revised on 2026-10-27, following the user’s decision ② on
2026-10-26**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly
driven test files** under `tests/` may be modified when mechanically required, provided that each
modification is registered with its **file / line count / rationale**, and **no test may be removed
and no assertion may be weakened**. The original criterion (`git diff --stat tests/ src/std/`)
imposed a zero-diff requirement on all of `tests/`; this revision relaxes it by one level for
directly driven test files only. The zero-diff requirement remains unchanged for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`).
**Exception**: the 6.8.2 behavior-fix batch is handled as a separate round with its own decision.

## Implementation Essentials

The acceptance criterion uniformly uses **C3 from 07: identical diagnostic codes and messages
(ordering may be normalized)**. The following is the stage breakdown for this workstream; see
RFC-039 for the global ordering across documents.

| Stage                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Rollback point                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2 (T3 landed in stage 0 and was later removed following the user decision on 2026-10-30). The first version of T1 **reports without failing**. Establish a full baseline on **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | The T1 report accurately lists the 13 **parser-forward zero-construction** variants (including the classification of `AssocType`: forward 0 / AST→AST 2, with D8 unchanged), and **separately reports the reverse-bridge reconstruction points from 2.2 and the AST→AST reconstruction points from 2.2.2**; forward zero construction of 13 variants is reported separately from global zero construction of 1 variant (`Enum` only); T2 is fully green or provides a repair list | No code changes; no rollback required                                                                                                                                                                                                                                                                                                                                                                |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) was consolidated to 15 names + the temporary `DateTime` alias, deleting 22 implementation spellings; the 7 lowercase duplicates in `add_builtin_types`, the `Uint` / `Int128` entries in the LSP list, and the `Uint` arm in `sizeof` were cleaned up in the same batch; **no lexer alias table was introduced** (the corpus contains 0 noncanonical names). **At the same time, verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**verified: it uses the unfiltered original signature list; after 6.5.2 it is provided by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                         | C3: the corpus-wide diagnostic set is **identical entry by entry** to the stage-0 baseline                                                                                                                                                                                                                                                                                                                                                                                        | Standalone commit; revertible                                                                                                                                                                                                                                                                                                                                                                        |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers changed to carry `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy bridge `From<MonoType> for IrType` (formerly `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias were removed (the `pub use` line at the head of `ir.rs` was reduced to a private `use`; that private `use` was subsequently removed with WBS 6.6.12); decoding now transfers values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), so the previous silent information loss in which 8 `MonoType` categories collapsed to `Void` no longer occurs                                                                                                                                                                                               | C3 + targeted `dump_bytecode` comparison (each row of the 6.2 table)—**measured fully green** (see the commit message for task 6.2)                                                                                                                                                                                                                                                                                                                                               | Standalone commit; revertible                                                                                                                                                                                                                                                                                                                                                                        |
| **3. Dead-variant removal**                   | **Completed (WBS 6.3, 2026-10-29)**: removed 13 variants according to the 5.2 table (`ast::Type` 26 → 13); removed match arms such as `mono.rs:699-708` and changed the `types.rs:836` fallback (to a let-else explicit return of `Type::Name{"()"}`); **handled `function.rs:507-529` (#47) and the 10 AST→AST points (6.3.2) in the same batch**; synchronized `types/tests/mono.rs`. **The criterion for #47 was corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” became invalid after 6.1—`type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes lowercase spellings it would become `TypeRef` and **silently lose type identity**. The actual implementation uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back | C3 + a fully green `cargo build` (exhaustive matches force every arm to be completed) + targeted generic-substitution comparison (last row of 6.4)—**measured fully green** (see the commit message for task 6.3)                                                                                                                                                                                                                                                                 | Standalone commit; revertible                                                                                                                                                                                                                                                                                                                                                                        |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: added `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); modified `parser_state.rs` (adding the `probe` field and the `probe()` / `kind_of()` accessors, with **all 40 construction sites connected** = 4 production + 36 test); deleted the 2 `operator_interfaces::spec()` calls in the parser and changed 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); consolidated the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connected `CONST_PARAM_TYPES` through the dual-knowledge `NameKind::Builtin` path (#50). **Not done**: adding a `kind` field to `Type::Name` at `types.rs:162-165` (#10; fieldization conflicts with this batch’s C3 surface), and changing the `types.rs:836` fallback to an explicit error return (#12, moved to 6.5)  | C3 + **new assertions**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps measured 0 matches** (`typecheck` / `operator_interfaces`); corpus 356/356, 216 zero-drift IR snapshots, and `cargo test --all` exit 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                              | Standalone commit; revertible                                                                                                                                                                                                                                                                                                                                                                        |
| **5. Dead AST variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: removed `Expr::FnDef` (baseline measurement: **25 locations / 16 files** = 1 definition + 15 production consumers + 3 test constructions + 6 comments, of which 19 were code references) + removed the parallel parameter tables in `Assign` / `TypeDefinition`; reshaped `Type::Fn.params` into `Vec<Param>` (names and types together) → the sole flattening entry point `Type::fn_params_flat`; retained `NamedParen` as a syntax node, with binder semantics explicitly interpreted by the type checker                                                                                                                                                                                                                                                                                                                                   | C3 + all **22 modules** in `tests/integration/` pass (measured 22 `.rs` files; the original text’s 18 was outdated); formatter parameter-position output remains byte-for-byte identical (355 `.yx` files compared byte by byte across the corpus); corpus 356/356, 216 zero-drift IR snapshots; `#[test]` count in `src/` 2878 → 2883 (no tests removed)                                                                                                                         | 7 commits (2 variant-removal commits + 3 representation-reshaping and old-representation-removal commits + 2 fix commits); **revert policy**: the variant-removal commit can be reverted independently, but the test-migration commit must be reverted with it (see “Registration ①” in the commit message for task 6.5); see the commit message for task 6.5 for measured results and gate outcomes |
| **6. Gates become hard failures**             | **Completed (6.6.2, user decision 2A, 2026-10-30)**: T1 changed from warning to `panic!`, and **T1.6 reconnaissance-baseline reconciliation was connected to the same failure path** (a construction-point count differing from `SCOUT_BASELINE` fails); in accordance with decision 1A, `without_fn_param_names` was classified in the second category, restoring the forward count to its frozen value; T4 was demoted to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | `cargo build` **must fail** when a zero-construction variant is deliberately introduced; changing any `SCOUT_BASELINE` variant count by 1 **must also fail**. Both mutations were measured using mechanically reversible patches (`scripts/dev/scratch.py trial`)                                                                                                                                                                                                                 | Standalone commit                                                                                                                                                                                                                                                                                                                                                                                    |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage 2**—before stage
2 is implemented, reverse bridge A on the bytecode side (`From<MonoType> for IrType` in
`bytecode.rs`) consumes `IrType::String` / `IrType::Bytes`, so removing the variants first would
break compilation. That bridge was removed in 6.2.2, meaning the remaining rewrite surface when
stage 3 removes the variants is reverse bridge B (`mono_to_ast_type` in
`middle/passes/mono/function.rs`, which still resolves `String` / `Bytes` by name) and the three
`ast::Type` carriers in `middle/core/ir.rs`. Stage 3’s #47 must be handled in the same batch as #1;
otherwise, `mono_to_ast_type` silently changes behavior. **Stage 3 was completed on 2026-10-29 (WBS
6.3)**: reverse bridge B was changed to use `canonical_name` + structural write-back; the three
`ast::Type` carriers in `middle/core/ir.rs` **were not changed**—6.3 only removed variants and did
not alter the IR shape (216 IR snapshots and the 356-line corpus baseline had zero drift), and the
carrier dispositions were **actually completed in 6.6.11 / 6.6.12** (two zero-producer dead variants
were deleted + `TypeDecl` was replaced with an IR-native narrowed carrier, and `ir.rs` no longer
imports `ast`). The original schedule that deferred this to stage 4 was not fulfilled; this
paragraph is authoritative.

**Downstream dependency**: after all stages in this article are complete, [04-ssa.md](04-ssa.md) may
begin. At that point, the element types of `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` are identical, and SSA type annotations have a single destination.

### Mandatory Validation for Each Stage

| Stage | Required action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction-point split**)                                                                                                                                                                                                                                                                                                                                  |
| 1-5   | `cargo test`; full corpus differential over `tests/yaoxiang/` (third layer of 07); compare every C3 criterion                                                                                                                                                                                                                                                                                                                                                                                      |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` output before and after the change                                                                                                                                                                                                                                                                                                                                                          |
| 3     | Additionally: select a corpus containing generic substitution and compare the monomorphized artifacts’ `type_name()` before and after the change—**the criterion was corrected (stage 3 measurement)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient for equality; judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()`, mutually inverse for every name) + structural comparison of written-back artifacts |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                                    |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                            | Rationale                                                                                                                                                                                                                                                                                                            |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` is the sole type structure + `MonoType` is the working representation; the `ir::Type` alias is removed                                                                                                                                         | The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) was the direct cause of the mismatch between the field types of `BytecodeFunction.params` and `FunctionCode.params` (2.1). After the alias is removed, the two field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                           | It would create an L2→L3 reverse dependency, violating RFC-039’s layering, and would require adding `Span` / `ConstExpr` back to `MonoType`. See the complete argument in 5.1                                                                                                                                        |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and move binder semantics to the **type-checking layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is the D1 target directory name, whereas the current repository directory is `typecheck/`) | `ast.rs:518-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as a binder                                                                                                                                                        |
| Directory rename             | **Perform it** (D1) as a pure move batch at the end of P5/P6; paths in this article’s body follow the pre-rename state                                                                                                                                | Decision of 2026-10-03 (ChenXu233); implementation details supplemented on 2026-10-04                                                                                                                                                                                                                                |

### Rejected Alternatives

None of the four alternatives was adopted: **retaining all three representations and adding a
conversion layer** resolves none of the root causes, and `ir::Type` remains a non-independent type,
with dead variants, the synonym table, and the reverse dependency all remaining; **generating the
synonym table with macros** treats the symptom rather than the cause—after `Type::Int(usize)` is
removed, a macro has no reason to exist, and consistency is instead guaranteed by gate T2 (more
verifiable than macro-expansion correctness); **removing only dead variants without unification**
has the lowest cost but leaves the problem in place, and without a gate the next person can add
`Type::Int128(usize)` and allow the same dead variant to grow back (if only one commit can be
delivered, it should explicitly be defined as a subset of this approach and include gate T1);
**unifying on `MonoType`** is discussed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. `Expr::FnDef` has 15 production consumers
  (including 3 exhaustive arms) + 3 test constructions + 1 variant definition, forming a parallel
  path that was never executed. After removal, `ir_gen.rs`, `ownership.rs`, and `termination.rs`
  each lose one branch.
- **Eliminates one duplicated implementation that has already diverged**. The two copies of
  `name_used_as_type` (2.4) differ in field recursion inside definition bodies; after consolidation,
  only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legitimate type value.”** This
  is the prerequisite for the “type consistency” invariant of the first-layer validator in 07 to
  pass.
- **Gates make regression impossible**. T1 turns “adding a zero-construction variant” from a silent
  pass into a build failure—the repository’s existing `build.rs` mechanism proves that this approach
  is feasible.
- **Zero source-code changes**. The 293 `.yx` corpus files and the standard library require no
  modifications; this is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may change behavior**. After the lossy bridge is fixed, the 8 `MonoType` variants expose
  their real types rather than `Void`. The direction is a fix, but it changes paths that depend on
  signature bytecode. This is the largest uncertainty in this article.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify variants. This is why manual verification is
  mandatory in stage 0.
- **If T1 fails to count reverse bridges, the disposition table and gate contradict each other**.
  Section 2.2 proves that 7 “dead variants” have production reconstruction points in reverse
  bridges. If T1 does not separate construction-point directions, these 7 variants will be
  classified as “having construction points” and exempted from deletion—the gate will become an
  obstacle to the deletion work rather than a safeguard. | **2. Deleting `signature_params` carries
  semantic risk** (**verified and resolved: 6.5.2, 2026-10-30**). It carries the **unfiltered**
  parameter list, whereas `Lambda.params` is **filtered**. If `classify_generic_params` depends on
  the former, the change would alter generic-classification results. **Measured conclusion**:
  `classify_generic_params` / `extract_generic_param_names` require the unfiltered list; flattening
  `Type::Fn.params` is derived from the same source as the old `signature_params` (the same `Param`
  nodes), so the classification result is unchanged. The approach of using `Lambda.params` would
  change behavior and **was not adopted** (corpus 356/356, 216 snapshots, and fully green
  `cargo test --all`). | **3. Stage 3 touches `middle/passes/mono/function.rs`**. In addition to the
  8 `AssocType` match arms, `507-529`, `mono_to_ast_type`, is the output path for monomorphized type
  substitution; after rewriting it, the substituted result’s `type_name()` may change (last row of
  6.4).
- **Noncanonical type names in the `.yx` corpus may be nonzero**. If the corpus extensively uses
  `i64` / `int`, stage 1’s “directly remove synonyms” approach is infeasible and would degenerate
  into “move them into a lexer alias table,” reducing the scope of `from_builtin_name`
  consolidation. (**Measured as 0**: the 6.0 reconnaissance in 5.3 shows this, so WBS 6.1.2 directly
  removed the aliases and did not take the alias-table route.)
- **The gate will initially be red** (**this risk was handled in two steps**). If T1 directly used
  `panic!` on its first release, it would immediately block compilation, so it first reported only
  and established the baseline from 6.0.1 onward; it became a hard failure in 6.6.2. By the time it
  became hard, the baseline had already been refreshed under the same criterion to 13/13 matching
  rows, and the current tree has 0 zero-construction variants.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1-D50) for the
> individual decisions. **This article leaves no pending items.**

## See Also

### Higher-Level and Same-Batch Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — Higher-level master plan; four-layer model, criterion tiers, and routing table C (3
  type-representation-related entries belong to this article)
- [01-routing.md](01-routing.md) — Feature routing tables A/B/C, dependency-direction rules, and
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; a prerequisite and downstream consumer of this article
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; consolidation of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definition of category C3, diagnostic-set
  comparison specification, and the type-consistency invariant of the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for the
  `Type::Generic` unified path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Generic mechanisms
  and type-level constructors (`If` / `Match`); **does not contain a built-in type-name list**, and
  is therefore not a gate-comparison target (the original T3 was removed following the user decision
  on 2026-10-30; see 5.7)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of a build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line-number anchors were refreshed to the state after 6.5** (2026-10-30); entries removed by
> preceding batches are retained and marked with their removal batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after 6.3; the
  26-variant era covered 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — Explanation of the binder semantics of
  `Type::NamedParen` (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types together) and
  `Type::fn_params_flat` (the flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- (**Removed in WBS 6.6.12**) `src/frontend/core/parser/ast.rs`, `Type::without_fn_param_names`: a
  printing-boundary consolidation introduced in batch 6.5 for “IR uses `ast::Type` as its carrier”;
  after 6.6.12 replaced `FunctionBody::TypeDecl` with an IR-native narrowed carrier, AST no longer
  enters the IR, leaving the function without a production caller, so it was deleted (T1’s
  `REBUILD_FUNCTIONS` / `SCOUT_BASELINE_CALIBER` lost one entry in the same batch)
- `src/frontend/core/typecheck/probe.rs` (since WBS 6.6.9) — `CONST_PARAM_TYPES`: the sole
  authoritative table of **acceptable type names for const generic parameters** (formerly in
  `parser/ast.rs`; it was the third copy of L3 knowledge. After the parser’s closed set was cleared,
  it moved back to L3, and the parser asks through `TypeEnvProbe::is_const_param_type`; see 5.4)
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single implementation after the
  6.4 consolidation (the former `name_used_as_type_in` and the `declarations.rs` copy were merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through the injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — The parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: fallback for non-`Fn`
  forms (since 6.3, returns `Type::Name{"()"}`; the `Type::Void` variant was deleted)
- (**Removed in 6.4**) `src/frontend/core/parser/statements/declarations.rs:42-80` — The original
  `name_used_as_type`, consolidated into the single implementation in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — The hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call (6.4 changed these to `ParserState::kind_of` / the
  injected probe; both literal references disappeared, leaving only the comment)
- `src/frontend/core/parser/pratt/nud.rs:512` — Expression position is represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — The `expr_to_params` arm recovers the parameter list
- `src/frontend/core/types/mono.rs:1123` — The `from_builtin_name` built-in-name parsing table (15
  names, no aliases—`DateTime` was removed with S11.1 (issue #513); finalized by WBS 6.1.2)
- `src/frontend/core/types/mono.rs:1216` — The `From<ast::Type> for MonoType` implementation (13
  match arms)
- `src/frontend/core/types/mono.rs:169` — Definition of `MonoType` (approximately 22 variants)
- `src/frontend/core/types/const_data.rs:240`, `:327` — `const_data::BinOp` / `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- (**Removed in 6.2.2**) `src/middle/core/ir.rs:3` — Evidence that `ir::Type` was an alias of
  `ast::Type`
- `src/middle/core/bytecode.rs:809` (`BytecodeFunction`), `:814` / `:816` (`params` / `return_type`,
  carrying `MonoType` since 6.2)
- (**Removed in 6.2.2**) `src/middle/core/bytecode.rs:2352-2390` — Lossy `From<MonoType> for IrType`
  bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `:275-278` — `BytecodeFile::type_table` and the
  `Vec<MonoType>` fields of `FunctionCode`
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: the reverse bridge after the 6.3
  rewrite (using `canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — The interpreter module’s `type_table`
- `build.rs:33-51` — Build-time error-code registry gate (the model for the type-table gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — The gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
