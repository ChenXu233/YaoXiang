# Unification of Type Representations

> **Supplementary design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-phase ordering are defined in the main
> RFC-039 text; the role of each companion document is described in the [directory index](index.md).

## Positioning and Scope

### Coverage

Consolidate the multiple parallel type representations currently in use into a single
representation, and eliminate the four categories of burden derived from them:

- **13 `ast::Type` variants with zero construction in the parser-forward direction** (among the 26
  variants in `ast.rs:427-541`, see 2.2; the “forward” criterion excludes reverse bridges and
  AST→AST reconstruction sites)
- **A handwritten table of synonymous type names** (`mono.rs:839-886`, see 2.3; WBS 6.1.2
  consolidated this into a 15-name parsing table)
- **The parser’s genuine reverse dependency on the typechecking layer** (2 sites, see 2.4)
- **One expression variant with zero construction in production**, `Expr::FnDef` (**deleted: WBS
  6.5.1, 2026-10-30**—the variant definition and every consuming arm were removed; see 2.6)

There are four concrete deliverables: a per-variant disposition table for all 26 variants (5.2), a
semantic classification for `Type::Name` through `NameKind` together with elimination of the synonym
table (5.3), an injected data flow through `TypeEnvProbe` (5.4), and build-time type-table gates
T1/T2 (5.7; the original T3 was deleted following the 2026-10-30 user decision).

### Directory Renaming and the Path of This Document

RFC-039 decision D1 mandates renaming execution directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`), **to be completed together with P5/P6, without splitting the work into
phases**. This document’s 50-item change list and all acceptance greps are written against the
**pre-rename** paths—they are the construction baseline for the P6 batches **before the rename
batch**; the directory rename at the end of each batch is a **pure-move batch** (C1 zero-diff)
committed separately. The baseline becoming invalid after the rename commit is expected; use git
history as the bisection basis.

### Out of Scope

- **The construction method for the directory rename**. During this phase, `Type` does not move out
  of `parser/ast.rs`; only `lower.rs` is added under `types/` as the sole conversion point. The
  directory rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 according to
  RFC-039 decision D1, in the form shown by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **The tiering of equivalence criteria and validator implementation**. The definitions of C1–C6 and
  the three criterion tiers (IR validator / normalized snapshots / corpus differential) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document cites only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite; the reason is
  given below.
- **The global P1–P10 execution sequence**. See the main RFC-039 text and
  [02-stage-contract.md](02-stage-contract.md). This document gives only the phase breakdown for its
  own workstream (see “Implementation Notes”).

### Division of Labor with RFC-039

RFC-039 is the overarching charter: it defines the four-layer model, criterion tiers, cross-document
phase ordering, and “why perform this refactoring now.” This document is the construction plan for
**the single type-representation convergence workstream** in RFC-039: which variants to delete,
which files to change, in what order, and which gates prevent regression.

Every line number, variant list, and construction-site count in this document comes from static
verification of `src/`; each item provides `file:line` evidence. **This document does not record the
execution process; it records code facts and the dispositions derived from them.**

To facilitate cross-references with documents in the same batch, the “Target Design” and “Detailed
Design” chapters retain their `5.x` / `6.x` subsection numbering; the remaining chapters follow this
directory’s standard style.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(the `pub use` at the head of `src/middle/core/ir.rs` before 6.2.2; that alias has since been
deleted, so it is no longer referenced by line number below). [04-ssa.md](04-ssa.md) will introduce
register type annotations and new instructions into the IR. Unless the representation is
consolidated beforehand, SSA type annotations will grow directly from the `ir::Type` alias onto the
syntactic type with 26 variants, effectively institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree inside a type.
  It is impossible for an IR instruction’s type annotation to contain an `Expr`.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and a `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently cement the 13 dead variants
and the two syntax-specific variants into the IR contract.

## Current State

Everything below is a statement of code fact and contains no proposals. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and Lossy Bridging

> **Status note (2026-10-28, WBS 6.2 landed)**: this section records the state **before** 6.2
> landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed four type carriers to carry `MonoType`, removed
> the lossy `From<MonoType> for IrType` bridge, and removed the `ir::Type` re-export alias; every
> statement below concerning `ir::Type` or the lossy bridge is therefore a **pre-change** fact, and
> locations marked “(deleted in 6.2.2)” describe an already-effective disposition. Implementation
> details and measured figures are in [09a](./09a-wbs-history.md) §P6 6.2.

The multiplicity of type representations is the common upstream source of four independent defects:

| Downstream symptom                                                               | Causal relationship to type representations                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, the parser selects the latter, and typechecking must supply a table to translate the former back                                                                                 |
| Bytecode-layer signatures differ from IR-layer signatures                        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias; consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one built-in type name requires changes in multiple places with no checks | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; each location writes the mapping separately and neither validates the other                                                                                                                   |
| The parser hardcodes `"Terminates"`                                              | Because the parser cannot determine “is this a type?” or “is this an operator interface?”, it must embed type-layer knowledge in the syntactic layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                             |

**The first three are representation problems; the fourth is a downstream consequence of a
representation problem.** Doing only item 1 (deleting dead variants) leaves the synonym table and
reverse dependency in place; doing only item 4 (injecting a callback into the parser) builds on an
inconsistent representation.

One of the three parallel representations is an alias illusion:

| Name        | Definition site                                                            | Size        | Role                                                    |
| ----------- | -------------------------------------------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541`                                  | 26 variants | Syntactic-layer type                                    |
| `MonoType`  | `src/frontend/core/types/mono.rs`                                          | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | Before 6.2.2: head of `src/middle/core/ir.rs` (that line has been deleted) | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

Before 6.2.2, the head of `ir.rs` first had a `pub use … parser::ast::Type;` line, immediately
followed by a `use … typecheck::MonoType;` line. The IR module depended on two “type” symbols at
once, although one was merely an alias. (Both lines disappeared when the alias was removed, so they
are no longer referenced by line number.)

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was deleted in 6.2.2).** `From<MonoType> for IrType`
(previously at `src/middle/core/bytecode.rs:2353-2390`) degraded `MonoType` to `ast::Type`, and its
**information loss was silent**:

```rust
// bytecode.rs:2371-2376
MonoType::Generic { name, args } => match name.as_str() {
    "String" => IrType::String,
    "Bytes" => IrType::Bytes,
    "Tuple" => IrType::Tuple(...),
    _ => IrType::Void,          // ← Any other generic type silently becomes Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

All 8 `MonoType` variants collapsed to `IrType::Void`, in addition to an `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; the comment itself admits that this was a “transition branch
before deleting the enum variants”).

`IrType::Void` is **a legitimate type value here, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function’s return type was lost
during bridging”—one reason why the first-layer validator in 07, the “type consistency” invariant,
cannot currently pass on this code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), precisely the dead variants addressed in 2.2—**both ends of the
lossy bridge point to dead code**.

**Current state (from 6.2.2 onward)**: the impl has been **removed entirely**, and decoding no
longer degrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now transports the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those 8
`MonoType` variants retain their true types in signatures and type tables, and `IrType::Void` is no
longer used as a substitute for “information loss.” The `ir::Type` re-export alias (the
`pub use ... ast::Type` line at the head of `src/middle/core/ir.rs` before 6.2.2) was also reduced
to a private import—the IR layer no longer publicly re-exports the syntactic-layer type. The IR
layer previously had three carriers that directly held `ast::Type` (`Instruction::Cast.target_type`
/ `Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`): the first two were deleted in
**WBS 6.6.11** together with their two **zero-producer** dead variants (the payloads were already
discarded downstream); the last was replaced in **WBS 6.6.12** with the IR-native narrowed carrier
[`TypeDeclBody`] (a `Vec<TypeDeclField>` field table and a `self_type: MonoType` bytecode type-table
entry)—**the IR layer no longer holds any `ast::Type`, and `src/middle/core/ir.rs` no longer imports
the syntactic-layer `ast`**. **The same-named bytecode-side `BytecodeInstr::Cast` is a separate
construct** (its payload is a type-table index `u16`, not a syntactic type) and remains unchanged.

### 2.2 Parser-Forward Zero-Construction Variants

**Scanning criterion**: the construction-site scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. **A construction site is a `Type::<variant>` path in expression position** (pattern
positions are not counted), and sites are **listed separately in three categories and must not be
mixed**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero construction in the parser-forward
direction** (reconstruction sites in the latter two categories are described in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                            |
| ------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                          |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                          |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                          |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                          |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an individual exemption; eliminated with 6.4 #12)                                                                                                                                                  |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                          |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Definition-column line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the
> “Definition” columns in this table and in 5.2 were written on 2026-10-05; 17 lines were
> subsequently inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so
> **the definition lines for all 26 variants uniformly shifted by +17** (for example, `Int` moved
> from 432 → 449 and `NamedParen` from 533-540 → 550-557). This round only **registers** the drift
> and does not alter the table body; subsequent references use the `ast::Type` symbol anchor.

The complete causal chain can be verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… they follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site for `Type::Void` is an error fallback**—`types.rs:836`
   contains `_ => (Vec::new(), Type::Void)` at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, and `types.rs:100` plus
`declarations.rs:573`, respectively. They are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document’s wording is “**zero construction in the parser-forward direction**”: it
means neither “zero references globally” nor “zero construction in production”—11 of these do have
production construction sites in the other two categories (reverse bridge / AST→AST reconstruction).
These tests themselves cement the behavior under test: they assert precisely what these syntactic
forms lower to as `MonoType`, even though the parser never produces them. When deleting the
variants, these tests must be deleted or rewritten as well.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The table’s “zero forward construction” result holds only in the **parser → AST
forward-construction** direction. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct 7 of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Members of the 13-variant set reconstructed                                                                              |
| -------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `From<MonoType> for IrType`                                    | `bytecode.rs:2353-2390`                  | `Int` (`2357`), `Float` (`2358`), `Bool` (`2359`), `Char` (`2360`), `Void` (`2361`), `String` (`2372`), `Bytes` (`2373`) |
| `mono_to_ast_type` (nested fn inside `substitute_type_in_ast`) | `middle/passes/mono/function.rs:507-529` | `Int` (`512`), `Float` (`513`), `Bool` (`514`), `Char` (`515`), `Void` (`516`), `String` (`522`), `Bytes` (`523`)        |

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

**This scope boundary directly affects disposition**:

- The `bytecode.rs` site disappears with #28 of 6.3 (deleting the entire `From<MonoType> for IrType`
  impl), so it adds no extra cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name decoder (marked `#299` in the comment), converting `String` / `Bytes` from `Generic`
  back into dedicated variants by name. After deleting `ast::Type::String` / `Bytes`, these two
  guard branches must be rewritten; after deleting `Int` / `Float` / `Bool` / `Char` / `Void`, the
  behavior of the `_` arm changes (some types that previously fell into `_` will silently switch to
  `AstType::Name`). This is item 47 added to 6.3.
- **Direct consequence for the gate**: a T1 gate that scans only forward construction sites would
  classify the 7 variants above as “having construction sites,” thereby excluding them from the
  deletion list. T1 must explicitly include reverse bridges in its counting criterion, or its
  conclusions will contradict the disposition table in this document (see 5.7).

#### 2.2.2 Scope Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution (reconstructing an
existing AST type, **not** `MonoType → AST`). The P6 T1 gate measurement (6.0.2 baseline,
2026-10-27) found **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to the
13-variant list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these 5 variants, it must rewrite these 10 sites in the same batch** (registered
in 09 under 6.3.2); otherwise, the corresponding arms will no longer compile after the variants are
deleted. **Implemented (WBS 6.3.2, 2026-10-29)**: all 10 sites were deleted; see the disposition
table in [09a](./09a-wbs-history.md) §P6 6.3.

### 2.3 Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`; measured at 841-867 / **13 arms**
when this section was written, and **839-886 / 15 arms / 16 spellings** after WBS 6.1.2) uses a
string table to map type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :843
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :850
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :851
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :863
```

**The sole reason this table exists is to bridge an inconsistency in the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but structurally
different at the AST layer (and the parser produces only the latter), `from_builtin_name` must
accept case variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"`
on the `Void` line is especially telling—it works with the `Type::Void` fallback in `types.rs:836`.

The same knowledge—which names are built-in type names—exists in seven copies throughout the
repository, with no consistency validation between them:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, so the original line numbers have drifted)             | Form                                                                                                                                                                                                                     |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                          |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, copied the three names `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                  |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | A `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                               |
| 4   | `src/lsp/world.rs:182`                                                                                                            | The LSP-side list of built-in type names (14 entries = the 15 names minus `()`; WBS 6.1.2)                                                                                                                               |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` aliases (registered by the 2026-10-06 P0 gate)                                                     |
| 6   | The base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                       | String match, name → byte size. It previously had a `"Uint"` arm (item 1 had no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **Dead arm, living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typechecking side’s treatment of “Set by name” (`inference/expressions.rs:2917-2918`, decision 4 of #300)                                |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223 lines and the others by 2–9 lines each),
> but **the number of entries and their contents have not changed** (item 1 had 13 arms when this
> section was written, item 3 has 15 entries, and item 4 had 15 entries when written; after WBS
> 6.1.2, item 1 has 15 arms / 16 spellings and item 4 has 14 entries). Subsequent references use the
> **symbol anchor + repository path**; the line numbers in this table are for location only. The arm
> count for item 1 and the living drift evidence for items 5 / 6 / 7 (the dead `Uint` arm and the
> contradiction between `Set` and the typechecking side) **all remain**, and this section did not
> change code.

`CONST_PARAM_TYPES` in item 3 is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to determine const generic parameters. It
enumerates “type names usable as const parameters,” which **partially overlaps with but differs
from** the complete set in `from_builtin_name`, and no mechanism guarantees that the two remain
synchronized. Item 4 is a fourth copy used for LSP syntax highlighting and hover information,
likewise without validation (T2’s comparison scope includes it).

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second synonym table for type names; items 6 / 7 share the same root as
items 1–4 (string-keyed tables mapping built-in names to semantics / IDs), although their purposes
differ and they share the same drift risk. The same gate also found 11 string dispatches on built-in
container names on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4,
`"String" | "Bytes"` ×4, etc.)—these are **outside both this section and the parser data-flow
coverage of 5.4**. The inventory and disposition are registered in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as P6 extended-verification items.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation`
/ `name_used_as_type`, both of which belong to the type layer). **Measurement differs from that
description**: both functions are defined inside the parser, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository’s only `fn`
  definition)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the repository’s only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production-code references that cross into `typecheck`**, and both call the
same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                               |
| --- | --------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Determines constraint-position parameters (`T: Add`), which do not occupy runtime parameter positions |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Determines constraint-position parameters and produces `GenericParamName` values with `constraints`   |

The second site is more deeply embedded—**`parser/ast.rs` itself** (the entire `ast.rs:847-948`
block, alongside `StmtKind`/`Expr` in the same file) directly references a typecheck symbol. Routing
table C records only the `declarations.rs` site.

Together with the hardcoding at `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Expanded as built-in predicates are added (future open-set approach: pass
// predicate names down to the parser, or use a form decidable at the syntax layer).
// The parser does not hold a type environment, so it looks in two places: built-in
// predicates are hardcoded, plus declarations parsed in this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment admits that this is a temporary form of the “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntactic layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask “is this the
name of an operator interface?” because it must distinguish whether `Type::Name` denotes a type or
an interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings** (concrete type / type variable / constraint name / predicate name), so the
syntactic layer cannot determine this itself and the decision authority is forced upward into the
type layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` acknowledges
this:

```rust
/// Whether a parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; implemented independently
/// here to avoid a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                         | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback             | No callback                                          |
| `Type::Generic` branch  | Short-circuits to `false` after `is_predicate(app_name)` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                          | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records how the latter two branches came to be added: “These two
branches were previously missing, so a const generic parameter referenced within a definition body
could not be detected as ‘used as a type,’ and annotation validation falsely reported a valid const
parameter as an unknown name.”

In other words, **the function was copied to avoid a cross-module dependency, but fixes did not flow
back to the original**. The `ast.rs` version recursively inspects fields in a definition body, while
the `declarations.rs` version does not—the same source fragment receives different conclusions at
the two decision sites. This duplicate must be consolidated when 5.3 introduces `NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums. They do not reference each other, and **their members have already
diverged**:

| Concept          | Syntactic layer                 | Constant-evaluation layer                                             | Difference                                                                                                                                     |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntactic layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntactic layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Both also independently implement complete `Display` implementations (`const_data.rs:291-317`) and
classification predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at
`const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in type position, they fall into
`Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly related to the
IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears following
`Type → MonoType`, and the IR never sees it, whereas `Assign` / `Deref` continue to exist through
paths such as `ir_gen.rs`’s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
Because the two enums have diverged, neither side alone can determine which operators may occur in
type position.

This phase does not merge the two enums (their ownership is discussed under “Changes to the Operator
Surface” in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate’s scan targets
should include `ast::BinOp` / `ast::UnOp`, making newly added zero-construction operators equally
visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction (Deleted: WBS 6.5.1, 2026-10-30)

**This section records an already-landed fact**: `Expr::FnDef` was once a variant of `ast::Expr`.
Its **zero construction in production** is accurate—there are only two construction sites in the
entire repository (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`), both in tests.

However, **there were far more than 4 consumption sites**: the baseline measured **15 production
consumption sites** (classified in 5.5), plus 6 comments mentioning the variant; in the 6.4 closing
state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1 definition + 15 production
consumption sites + 3 test construction sites + 6 comments.

**The path actually taken by function definitions** was: `pratt/nud.rs` constructs `Expr::Lambda`,
and `parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in
`value`). In other words, the `FnDef` branch was **a never-executed parallel path maintained at 19
code locations** (15 consumption sites + 3 test construction sites + 1 definition).

**Landed state**: the variant definition and every consuming arm were **removed**, and
`git grep -n "Expr::FnDef" -- src/` inside `src/` returns 0 matches (only 2 explanatory comments
containing the bare word `FnDef` remain). See the per-site disposition table, test migration, and
measured figures in [09a](./09a-wbs-history.md) §P6 6.5; see 5.5 for the design of the merge.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` once carried a typechecker-specific field (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>`
(originally `ast.rs:244-245`), and its comment stated outright: “The first group of signature
parameters is preserved as-is (including parameter names), for the typechecker’s
`classify_generic_params` use”—the statement AST reserved a dedicated slot for the type checker.

Current representation: parameter names and types **co-reside** in the type annotation’s
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` now carries the declared
signature `type_annotation: Option<Type>` (the same carrier as `Assign`).

**Second, expression position was used as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and uses it as a parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module’s match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:518-532` states explicitly:

> RFC-027 §3 return-position refinement relies on it to declare the **return formal parameter name**
> (called a binder here)… **Without it, the type checker can only guess** that “a free variable in a
> constraint that is not in scope is the return-value parameter,” so `(r: P(m))` would silently
> substitute the undeclared `m` as a binder as well (observed defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Claim: a single representation means `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is demoted to the type checker’s internal working representation; the `ir::Type` alias is
deleted.**

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

**Why not “merge into `MonoType`”** (this is the easiest alternative to propose, and it is rejected
here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), whereas
   `MonoType` does not. C3 in 07 requires “the same diagnostic code and message”—diagnostic
   positions must remain traceable to source. Adding `Span` to `MonoType` would attach a pile of
   meaningless sentinel spans to every monomorphized type.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker’s solving
   process**, not types themselves. Putting them in the sole type structure would force the
   IR-construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), lowered away in `MonoType`. Unifying on `MonoType` would
   require adding these syntactic forms back to it.
4. **The decisive counterargument**: elevating `MonoType` to the sole representation would make the
   L2 syntactic layer depend on the L3 semantic layer’s type. RFC-039 explicitly forbids L2 from
   depending on L3. Eliminating reverse dependencies is one of this document’s goals, so choosing a
   solution that creates a reverse dependency would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) caused
`BytecodeFunction.params` (`bytecode.rs:812`) and `return_type` (`bytecode.rs:814`) to use
`ir::Type`, while the serialization side used `MonoType` at `codegen/bytecode.rs:277-278`. The two
field types were connected by `file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339`
and the lossy `From` bridge at `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the two sides have different field types.”**
Once `bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly
changed to `MonoType`, `From<MonoType> for IrType` has no reason to exist and can be removed
entirely (which also eliminates the reverse reconstruction recorded in 2.2).

**Explicitly out of scope**: do not move `Type` from `parser/ast.rs` to `sema/types/`. This phase
only adds `lower.rs` under `types/` as the sole conversion point and leaves the definition of `Type`
in place; the directory rename is performed at the end of P6 as a separate pure-move batch under
D1—afterward, `Type` belongs at the top-level `ast/type_.rs` (the AST top-level domain), while
`sema/types/` is the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> **Landed (WBS 6.3, 2026-10-29)**: all 13 “Delete” items in this table **have been
> implemented**—`ast::Type` was consolidated from 26 variants to 13 (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were rewritten in the same batch. See the measured figures, the 10-site disposition
> table, and the criteria in [09a](./09a-wbs-history.md) §P6 6.3.

> Disposition categories: **Delete** (zero construction in the parser-forward direction), **Retain**
> (has forward construction sites), **Migrate** (semantics move to another node).

> **Line-number convention**: the “Definition line” column contains the **original positions before
> variant deletion** (the 26-variant era; 6.3 deleted 13 of them); the matching-site line numbers in
> the “Rationale” column use the same **pre-6.3 baseline** and do not shift as code moves—this batch
> only refreshes the definition lines for the **still-existing variants** (rows 14 / 26) to their
> landed positions and corrects the now-invalid consumption site in row 14.

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthen)                    | The parser’s **only** primitive-type exit (`types.rs:162-165`). After deleting the 12 primitive-type variants, all primitive types use it. A semantic-kind annotation for the name must be added (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:699` is a match arm). Bit width is carried through the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:702` is a match arm). **Note**: `IrType::String` at `bytecode.rs:2372` consumes it; that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:703-706` is a match arm). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The only parser-forward construction site is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an individual exemption). The fallback is eliminated together with the 6.4 error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction site `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:743-746` is a match arm); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:747` is a match arm); all three categories total 0 (the only globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; this is unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                                 | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering); the originally cited `bytecode.rs:2366` referred to `IrType::Fn` and disappeared with the 6.2 removal of the `ir::Type` alias and lossy bridge. **Field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types co-reside) follows 6.8.1 (D54)**—the names formerly in `Assign.signature_params` move here (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous`/`from_type`/`is_named`; unnamed parameters (pure type positions) are synthesized with empty names, leaving this table’s “net effect” unchanged; `Param` has a handwritten `Debug` implementation (unnamed parameters print according to their carried type ⇒ zero IR-snapshot drift, see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that it uses the `Generic` path                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero construction in the parser-forward direction (the syntactic layer has no `::` path that produces it); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered under 6.3.2). **D8 unchanged**: the criterion is forward construction; if associated-type syntax is enabled in the future, a new proposal at that time will define it again                                                                                                                                                                                                                                                                                                                                                                     |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero construction in the parser-forward direction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction site `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type in unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has production construction site `types.rs:160`. It is the only variant in `Type` that embeds an `Expr`; the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Its semantics (the binder name) move to the type checker; the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now states “parameter names belong to `Type::Fn.params`; this node only carries the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **The reverse bridge and AST→AST reconstruction must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs 7 of rows 2–8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 includes 5 variants / 10 sites
> from the 13-variant list (Union/Option/Result/Sum/AssocType, 2 each). Both categories of
> reconstruction site must be processed in the same batch as 6.3.1; otherwise, behavior either
> changes silently (see 6.3 #47) or the code no longer compiles.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly under D8
> (5.2, row 18). The **criterion is parser-forward construction**: zero forward construction holds
> (the syntactic layer has no path that produces `::` associated types), so D8 need not be changed
> and the variant does not enter the whitelist. **The 2 production AST→AST reconstruction sites**
> (`function.rs:456`’s `substitute_type_ast` and `function.rs:623`’s `substitute_type_in_ast`) were
> registered under 6.3.2 and rewritten in the same batch as 6.3. T1’s three-category classification
> (5.7) reproduced this conclusion (all 26 variants matched per variant in the 6.0.1 commit).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine “is this a predicate?”,
the same file at `510-512` must determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` must determine “is this an operator interface?”, and the two
`name_used_as_type*` implementations at `name_used_as_type_in` (measured 864) in the same file and
`name_used_as_type` at `declarations.rs:42` must determine “is this a type reference?”—all through
string comparisons.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target shape (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → delegate to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: the information required to classify `NameKind` is exactly what the parser
currently lacks** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be classified independently inside the parser**—it
must be classified through an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (landed in WBS 6.1.2, 2026-10-27, final user decision)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) **is retained, with
reduced semantics**. Before the refactoring, it was a disambiguator that bridged two equivalent AST
representations; after `Type::Int(64)` was deleted, `"Int"` had only one representation path, and it
became the **sole name→type parsing table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language built-ins
  (`src/std/*.rs` signature strings, `CONST_PARAM_TYPES`, and the LSP list all use it as their
  reference).
- The `DateTime` arm records a real fix (#338 related issue: `now()`’s return value could not be
  passed to `format_time`)—it is a **semantic alias**, not the same kind of case alias as
  `Int`/`int`, so it is retained separately until P10 S11 (issue #513).

**Result** (final criterion = 15 built-in names + temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                     | Before                                           | Disposition                                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Crowded into 13 arms together with aliases       | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                         |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                         | **Delete** (22 spellings), **with no lexer alias table**—corpus measurements found 0 noncanonical names (see the design judgment at the end of this section), so syntactic-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | On the same arm as `Void`                        | **Retain**—`()` is the type spelling of `Void` (standard-library signature strings write `() -> Void`), not another type                                                                                      |
| `DateTime`                                                                                                                                                                                                                                   | `datetime` and lowercase aliases on the same arm | **Retain only the uppercase** temporary compatibility alias, noting that its migration belongs to P10 S11 (issue #513)                                                                                        |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares the
`Void` arm; there is also one temporary `DateTime` alias). After deleting the 22 spellings, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—it is **no longer silently accepted**.
Previously, `add_builtin_types` also registered lowercase duplicates of `int` / `float` / `bool` /
`string` / `void` / `never` / `char` in the type table, constituting a second alias table; these
were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for the 22 spellings, and `DateTime` → `Int(64)`);
2. T2.5, the “Deleted-Spelling Sentinel,” in `tools/type-tables --report`—if any of the 22 spellings
   is added back to the parsing table, the report immediately counts it as **unregistered drift**
   (there is no disposition to register: that would be a rollback).

**Consolidating the two `name_used_as_type*` implementations**: the duplicate implementations
recorded in 2.4 must be merged into one, or the classification information provided by `NameKind`
will be bypassed by two independent string-comparison implementations. The merged implementation
belongs in `parser/ast.rs` (the data source is next to `Type`), and `declarations.rs` should call
it; the `is_predicate` short-circuit should read `NameKind::Predicate`, and the `Struct` /
`NamedStruct` field recursion should follow the version at `ast.rs:894-905` (the newer and more
complete one).

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter-name table in `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps with the complete set in
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured 932) can be changed to “`kind == Builtin` and the
name is in the accepted const-parameter set,” explicitly connecting the two sources of knowledge;
the subset definition for const parameters is validated by the T2 gate (see 5.7).

> **Design judgment**: if `.yx` corpus files contain type annotations such as `i64` / `int`,
> normalizing them to `Int` changes the **diagnostic positions** of those files (but not their
> diagnostic codes). This is permitted under the C3 criterion (messages and ordering may be
> normalized), but the baseline must be recorded in phase 1. **Measurement (P6 6.0, 2026-10-27)**: a
> full scan of `tests/yaoxiang` (316 `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **0
> noncanonical names in real type positions**; 16 raw-word matches were adjudicated individually as
> 12 strings + 4 comments (both `i64` / `f64` occurrences were in comments). Therefore, 6.1’s name
> normalization **does not change the diagnostic position of any corpus file**, and **no baseline
> regeneration is required**.
>
> **Retest (P6 6.1 closeout, 2026-10-28)**: 6.1.1 rescanned the final **22** spellings—the
> 2026-10-27 measurement used a **narrow criterion** (counting only the 14 integer/float-family
> spellings and not namespace usage in `src/std`), which is why it recorded “16 raw-word matches =
> 12 strings + 4 comments.” The retest scope was **393 files**: 316 `.yx` files in `tests/yaoxiang`,
> 39 in `tests/yaoxiang-multifile`, and 38 in `src/std`; it found **308 matching lines**. **Only 2
> lines were type-position candidates, and both were inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5` with `i64::MAX`, and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17` with `-> f64`) ⇒ **there are still 0
> real type positions**; all other matches were adjudicated individually as string literals /
> comments / `std.string` module names and namespace access / `Json` variant names / `void`
> literals, and **none was a type annotation**. See the 6.1.1 row in [09](09-execution-wbs.md) and
> “P6 6.1 Name Normalization Closeout” in 09a for the retest command. The conclusion is consistent
> with 6.0: 6.1’s name normalization **does not change the diagnostic position of any corpus file**,
> and **no baseline regeneration is required**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed loop inside the parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injected, no boundary crossing):

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

| File                                                                                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                     | Define `trait TypeEnvProbe`, providing the four pure query methods `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is legitimate dependency inversion. **The landed method names are those in this table** (the original design’s `is_type_var` / `is_builtin_type` were not adopted: a type variable’s identity is determined structurally by its declaration site, not by its name, so no such criterion is provided; built-in-name classification is instead supplied by `kind_of`, with `knows_builtin_types()` declaring whether the probe holds the built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                              | Retain `declare_predicate` / `is_predicate_name` (for accumulation during the current pass); add a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**borrow rather than `Box`**). **The landed state also adds a `kind_of()` accessor**—predicates declared in the current pass take priority, and the remainder is delegated to the probe; this is the sole entry point for name classification                                                                                                                                                                                                                                                      |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                 | Delete `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                 | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/parser/ast.rs:930`                                                                         | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and should not hold an environment dependency—this function must accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                                              |
| All **40** parser construction sites (`ParserState::new` is the sole constructor; **4 production + 36 test**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. **The default implementation, `NullProbe` (all results false),** ensures parser unit tests remain unaffected; the default production-entry probe is `BuiltinProbe` (the compiler’s built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (**The original design’s “5 construction sites” was a pre-survey estimate; the measured number was 40**—`Parser::new` does not exist, and `ParserState::new` is the only constructor)                                                                                                                                           |

**The `extract_generic_param_names` signature problem**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that accepts no context. After deleting the boundary-crossing call at `ast.rs:930`, it
needs the `is_constraint` decision, so its signature must gain `probe: &dyn TypeEnvProbe`. Callers
(including `parser/tests/ast.rs`) must pass `&NullProbe` accordingly.

**When `NameKind` is classified**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, classify it once, and ensure that all later match arms no
longer perform string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them only involved parser-internal
functions under the criterion in 2.4, while the genuine cross-layer references are the 2 sites at
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Merging `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Claim: delete `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This was already the **active path** (`pratt/nud.rs` constructs
`Expr::Lambda`, and `parser/statements/declarations.rs` constructs `StmtKind::Assign`; see 2.6); the
`FnDef` branch was a never-executed parallel implementation.

**Landed state**: the variant definition and all **15** production consuming arms were **deleted in
the same batch** (Rust’s exhaustive `match` makes “definition + arm completion” an atomic unit; see
“Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5). Baseline consumption sites are classified
by form:

| Category                                                                       | Count | Disposition                                                                                                    |
| ------------------------------------------------------------------------------ | ----- | -------------------------------------------------------------------------------------------------------------- |
| Variant definition                                                             | 1     | Delete                                                                                                         |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (otherwise compilation fails)                                                                           |
| Non-exhaustive match arms + or-patterns with `_ => Span::dummy()`              | 10    | Delete (after deletion, control falls through to the fallback; retaining them would preserve a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after deletion, they never match and produce neither an error nor a warning)                           |
| Test construction sites                                                        | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not delete the cases)                                                 |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally equivalent to the first three fields of `FnDef`; the `name` field unique to `FnDef` has
no use at the `Expr` layer (the name lives on `StmtKind::Assign.target`).

**Risk and mitigation (originally planned, now landed)**: after the refactoring, the 2
non-exhaustive `if let` sites never match, with no error or warning. Mitigation is **compile-time
assurance**: after the variant is deleted, any new `if let Expr::FnDef` fails to compile directly
(the variant does not exist), so **no gate is needed**.

**Per-site disposition table, test migration, and measured figures**: see
[09a](./09a-wbs-history.md) §P6 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**Delete `signature_params: Vec<Param>` from `StmtKind::Assign` / `StmtKind::TypeDefinition`.**

Rationale: its reason for existing (the comment originally at `ast.rs:244`) was “for the
typechecker’s `classify_generic_params` use”—a **dedicated slot reserved by the statement AST for
the type checker**, redundantly transporting a second representation of the parameters.

**The implementation criterion differs in one important way from the first draft of this section**
(corrected after phase 1 verification): the first draft proposed “recompute the data in place from
`Lambda.params`,” but `Lambda.params` contains the **filtered** value-parameter list (type-position
and constraint-position parameters removed), whereas `classify_generic_params` /
`extract_generic_param_names` require the **unfiltered** original signature list. The actual
implementation therefore moves the names into the sole carrier before deleting the old field:

1. 6.8.1 first reshapes `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   co-reside, reusing the existing `Param`—no second structure and no parallel arrays);
2. 6.5.2 then deletes both `signature_params` fields: `Assign` obtains them from `type_annotation`,
   while `TypeDefinition` carries the declared signature `type_annotation: Option<Type>` (the same
   carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (expanding each
   curried group in order, treating `Paren`/`NamedParen` as transparent); the `NamedParen` binder is
   not included and is interpreted explicitly by the type checker (see below).

> **Risk-verification conclusion**: the difference between “unfiltered” and “filtered” identified in
> the first draft is real—but **not a problem**: the flattened result and the old `signature_params`
> are element-for-element from the same source (the same `Param` nodes), so generic classification
> does not change. The evidence is 356/356 corpus files, 216 zero-drift IR snapshots, and a fully
> green `cargo test --all` (transcript in [09a](./09a-wbs-history.md) §P6 6.5). The first draft’s
> “use `Lambda.params`” proposal would have changed generic-classification results and **was not
> adopted**.

**Retain the `Type::NamedParen` (`ast.rs:533-540`) node, but move binder semantics out of it
(completed: 6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records its **sufficient reason for existence** (see 2.7): in
`(r: P(m))`, dropping `r` would leave the type checker to guess that “a free variable in a
constraint that is not in scope is the return-value parameter,” which would **silently substitute
the undeclared `m` as a binder as well** (the comment marks this as an “observed defect”).

Therefore, it **cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param`/`param_span`
   carry only source facts and do not bear type-layer binder semantics.
2. Move binder interpretation to the **typechecking layer**: when the type checker consumes
   `NamedParen`, it **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node’s
   field,” the same kind of relationship as `ConstExpr` embedding an `Expr`: the syntax node
   provides facts, and the semantic layer interprets them.
3. Rewrite the comment from “the type checker can only guess” to “parameter-position formal
   parameter names belong to `Type::Fn.params`; this node only carries the **return-position**
   binder.”

> **Path correction**: the `sema/` in the first draft of this section is the **target** directory
> name from RFC-039 D1; the semantic-layer directory in the current repository is
> `src/frontend/core/typecheck/` (the rename belongs to the 6.7 batch and has not yet been
> performed). All code paths in the body of this document follow the current pre-rename paths (see
> “Directory Renaming and the Path of This Document”).

**Why this is a “migration,” not a “deletion”**: the syntax tree must represent source code
losslessly. `r` genuinely exists in the source, so the AST cannot pretend otherwise. The problem is
not the node’s existence, but the failure to assign responsibility for semantic interpretation
explicitly.

### 5.7 Build-Time Type-Table Gates

**The reference implementation has already been verified in this repository**: `build.rs:19-37`
calls `tools/code-tables`’s `parse_registry` / `validate` to check uniqueness, section bit
allocation, and Chinese-language completeness for 145 error codes; when `is_ok()` is false, it
directly calls `panic!` and rejects the build. `build.rs:39-54` then compares the markdown code
table from RFC-013 entry by entry, also calling `panic!` on a mismatch. **RFC-013’s documentation
and code therefore remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one
`serde_json` dependency, and the validation logic is text-level parsing that does not depend on syn.

The type tables currently have **no** comparable gate. The mechanism introduced here can directly
reuse that form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). The checks are as follows (the T3 column was deleted on 2026-10-30; the deletion
record is retained):

| #                                                                   | Check                                                                                                                                                                                                                                                                                                                                                                                                                     | Inputs                                                     | Failure condition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Remedy                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site classification**                             | Scan the variant list of `pub enum Type` in `src/frontend/core/parser/ast.rs`; under `src/`, count construction sites for each variant separately in **three categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`). Reconstruction sites (②③) are driven by the `REBUILD_FUNCTIONS` table in `scan.rs` and reconciled line by line with the source | Variant list + three-category hit tables + survey baseline | ① A variant has **zero forward construction** and is not registered; ② a registered variant now has a forward construction site; ③ a registered wildcard-arm fallback exemption no longer matches; ④ an **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered individually and must not be generalized to every wildcard arm); ⑤ **Survey baseline (T1.6) drift**: a variant’s total/forward/AST→AST/reverse-bridge counts differ from `SCOUT_BASELINE` (decision 2A); ⑥ **the reconstruction-site registry disagrees with the source**: a registered reconstruction function has no same-named `fn` in the source (rename/deletion was not synchronized); ⑦ **criterion and baseline are from different generations**: the current `REBUILD_FUNCTIONS` differs from the frozen criterion snapshot `SCOUT_BASELINE_CALIBER` (criterion revision) (6.6.3) | Delete the variant; or register a rationale / remove an invalid registration (see 5.2 and `REGISTERED_ZERO_FORWARD`); refresh `SCOUT_BASELINE` with reproducible evidence (including construction-site file:line); changing the criterion requires refreshing the criterion snapshot and baseline figures in the same batch; keep the registry synchronized with the source |
| **T2 Synonym table and repository-wide name-list consistency**      | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `typecheck/probe.rs` (moved back from `parser/ast.rs` to L3 in WBS 6.6.9) + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                                                                                                     | Three tables                                               | A type name exists in one table but is not recognized by another (compared according to each table’s semantic mapping rules); **from WBS 6.1.2, T2.5, the “Deleted-Spelling Sentinel,” additionally reports an error if any of the 22 deleted spellings is added back**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | `type-tables --fix` fills the gaps in both directions                                                                                                                                                                                                                                                                                                                       |
| ~~**T3 Built-in type names consistent with the RFC-011 document**~~ | Originally planned: the canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                                                                                                                  | —                                                          | **Deleted following the 2026-10-30 user decision (the comparison target does not exist)**: RFC-011 is a generics-system design and its body has no table with “built-in type names” as a column                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | — (not applicable; see the explanation after the table)                                                                                                                                                                                                                                                                                                                     |
| **T4 Exhaustiveness of `Type → MonoType`**                          | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                                                                                                                                                            | Two tables                                                 | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Add an arm / delete an arm                                                                                                                                                                                                                                                                                                                                                  |

> **Why T3 was deleted and the scope of T2 (2026-10-30 user decision)**: T3’s **comparison target
> does not exist**—RFC-011 is a generics-system design whose body has no table with “built-in type
> names” as a column. The first implementation had to use “every inline-code-wrapped token in every
> markdown table row whose entire token starts with an uppercase letter” as the document-side
> criterion, which necessarily produced a “no action required” empty-shell difference (16
> registrations: 3 document-only + 13 code-only, none of which could be adjudicated as true or
> false). Therefore, the check, its 16 registrations, and the corresponding output sections for
> `--report` / `--json` were deleted in the same batch, with no placeholder left on the code side.
>
> **The only carriers of built-in type names are the three code-side lists, all of which are now
> reconciled name by name by T2**: ① `from_builtin_name` (`src/frontend/core/types/mono.rs`; 15
> canonical names + the temporary `DateTime` compatibility alias, with the first spelling of each
> arm canonical; the 22 spellings deleted from the implementation in 6.1.2 are guarded by the T2.5
> sentinel); ② `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`; semantically a **subset of
> ①**, check C1); ③ the LSP built-in type-name list (`src/lsp/world.rs`; semantically a **superset
> of the canonical names in ①**, checks C2 / C3). No equivalent document-side list is established:
> the authoritative landing point for a new built-in name is ①, and T2 reports gaps in ②③ as
> **unregistered drift**.

**T4 implementation note**: Rust’s exhaustive match already guarantees the “variant → arm” direction
at compile time. T4’s value lies in the **reverse** direction—detecting stale arms in `lower.rs`
that point to **deleted variants** (such code would fail to compile when the variant was deleted, so
T4 is effectively redundant). **T4 is therefore downgraded to a CI assertion** (a successful
`cargo build` is sufficient), and it is not placed on `build.rs`’s `panic!` path. **T1/T2 are the
two items that genuinely require text parsing** (T3 has been deleted, as described above).

**T1 failure modes (6.6.2, 2026-10-30 user decision 2A)**: T1 changed from “report only” to a **hard
build failure**—`build.rs` calls `type_tables::build_gate(root)`: `Ok(summary)` prints one
`cargo:warning` line (an informational line, not a violation), while `Err(details)` directly calls
`panic!` and rejects the build. **All six violation classes use the same failure path** (①–⑤ and ⑦
in the table above; ⑥ belongs to the scanner-level failure path “reconstruction-site registry
disagrees with the source,” and, like the 6.6.3 criterion-generation mismatch, directly fails, but
is not counted as a gate-violation class). Every error includes the **variant name** and the
construction-site / fallback-site / variant-definition-line `file:line`; baseline drift also
includes these locations (following the 6.6.3 review F1: previously only the counts were provided,
leaving the reader unable to determine where to look). If the scan itself cannot complete (file
parse failure or incomplete criterion), that is also a failure—**it must not be downgraded to
“pass.”** **The registration mechanism is retained with empty tables**: `REGISTERED_ZERO_FORWARD`
and `DEAD_VARIANTS_03` are now both empty (after 6.3 deleted the 13 dead variants, there are no
zero-construction variants). T1.7’s “registered but no longer valid / exemption did not match”
warnings and injectable verification (`*_with(registry)`) continue to work—any new zero-construction
variant must be deleted or have a rationale registered in the table, or the build fails.

**Single source of reconstruction locations (6.6.2, user decision 1A)**: classification of the
second category and reverse bridges is driven by the `REBUILD_FUNCTIONS` table in `scan.rs`
(**file + function name → category**). `Type::without_fn_param_names` (the fidelity-preserving
reconstruction introduced in the 6.5 batch: it reconstructs an existing AST type to strip
type-position parameter names; its sole consumer normalizes the AST before entering the IR in
`ir_gen.rs`; **it was deleted in WBS 6.6.12 along with the narrowed IR carrier**, and its
registration and criterion snapshot were removed in the same batch) belongs to the **second
category, AST→AST reconstruction**. Previously, it was counted as parser-forward, artificially
adding 1 to the forward count of 11 variants (`Name` / `ConstExpr` use `self.clone()` and did not
produce sites anyway). The registry is reconciled **line by line** with the source: if a registered
function has no same-named `fn` in the source, the scanner reports an error immediately and **must
not silently fall back to forward classification** (otherwise, after a rename/deletion, a
reconstruction site could masquerade as a construction site).

**The baseline pins down two things (6.6.3 review F1/F2)**:

1. **The source tree**—`SCOUT_BASELINE_REV`: only the same source tree supports a meaningful
   comparison of site counts;
2. **The criterion**—`SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` (= the frozen snapshot
   of `scan.rs::REBUILD_FUNCTIONS` at that revision, recorded as “file + function → category key”).
   The snapshot deliberately **does not share a table with the registry**: if they shared one, both
   would change together and the gate could never detect a criterion change.

The gate performs machine reconciliation of the snapshot and the current registry
(`baseline::caliber_drift`, modeled on `scan::missing_rebuild_functions`): adding/removing a
reconstruction function, or changing a function’s category, reports **“criterion and baseline are
from different generations”** (including the criterion revision and each difference), and **skips
baseline-drift comparison**—counts across different criteria are not comparable, and continuing
would disguise “the criterion changed” as “the numbers drifted.” The `--report` T1.6 section prints
the baseline revision and **criterion revision**, so readers of the report or script runner can
reproduce the calculation using both revisions.

**Two implementation difficulties in T1 and their mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has only
   `serde_json`. Mitigations:
   - **Preferred**: introduce `syn` to `tools/type-tables` (only that crate; the root crate remains
     unchanged). `syn` is the standard approach, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the line-level heuristics of the existing
     `tools/extract_arm.py`—construction sites are on the right-hand side of `= ` or in
     function-argument position, while match arms are on the left-hand side of `=>`. Use the number
     of `=>` occurrences as an approximation.
   - **Conservative starting point (followed in this order)**: when T1 first launched, it **reported
     only and did not fail** (`cargo:warning`), allowing a full baseline to establish that the
     heuristics were sound before switching to `panic!`—from 6.0.1 it only reported, and on **6.6.2
     (2026-10-30) it switched to `panic!`**. By the time it became hard, the baseline had already
     been refreshed under the same criterion to match all 13 rows. This is consistent with handling
     “a CI gate may initially be red; establish a baseline first.”
2. **Reverse bridges and AST→AST reconstruction sites must be classified separately**. 2.2 records
   two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and `function.rs:507-529`;
   2.2.2 records the two replacement functions in `function.rs` as AST→AST reconstruction (15
   variants / 28 sites, including 5 variants / 10 sites from the 13-variant list). If T1 mixed these
   with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` and
   `Union` / `Option` / `Result` / `Sum` / `AssocType` would be classified as “having construction
   sites” and exempted from deletion, directly contradicting the disposition table in 5.2. T1’s
   report must list the three construction-site categories separately, and **fallback exemptions
   must apply only to individually registered entries** (currently only the parser error fallback
   for `Void` at `types.rs:836`), never to every wildcard arm.

**Gate coverage (landed in 6.6.2, 2026-10-30 user decision 2A)**: `build.rs` calls
`type_tables::build_gate(root)` immediately after the `code-tables` validation block, using the same
`panic!`-based build-rejection form—`Ok(summary)` emits one `cargo:warning`, while `Err(details)`
invokes `panic!("Type-table gate T1 failed (build rejected): …")`. **All six violation classes use
the same failure path**: zero-construction variant not registered / registered but now has a forward
construction site / registered exemption site did not match / unregistered wildcard-arm fallback
site / **criterion and baseline are from different generations** / **survey baseline (T1.6) drift**.
Every error includes the variant name and the construction-site, fallback-site, or
variant-definition-line `file:line` (drift entries have included construction-site locations since
6.6.3). If the scan itself cannot complete (for example, a file parse failure), that is also a
failure and **must not be downgraded to “pass.”**

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                                                                             |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                               |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `eval/reducer.rs:572`’s `unify` all operate on `MonoType`                                                                                                                                                                                                                       |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. The 8 matches of `AstType::NamedStruct` / `AstType::AssocType` at `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623` were synchronized—the disposition of `AssocType` was deletion, so all 8 were updated. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see 6.3 #47; it now uses `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match was consolidated with 6.3; the former direct read of `signature_params` as `let params: Vec<MonoType>` was replaced with flattening through `Type::fn_params_flat` (the verification conclusion in 5.6: an unfiltered list from the same source as the old field)                                                                           |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Requires change**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and the assembly at `632`                                                                                                                                                                                                                                                                      |
| Interpreter (`backends/interpreter/`)                        | **Requires change**. `image.rs:44` changes `type_table: Vec<ir::Type>` to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects the `{:?}` output format                                                                                                                                                                                    |
| RFC-027 compile-time types                                   | **Requires change**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible in the IR                                                                                                                                                                                                                                |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by the “type consistency” invariant in 07’s first-layer validator. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are the expression-side counterparts of the same problem—they may occur
inside `ConstExpr`, and therefore must not descend into IR type annotations either.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (the sole
   forward site for `Void` was an error fallback—**WBS 6.3 changed that fallback to
   `Type::Name{"()"}` and removed the gate exemption registration**, so there are now no forward
   construction sites) ⇒ the parser does not produce them (the reverse bridge and AST→AST
   reconstruction dispositions are described in 6.3 #28 / #47 and the 10 sites in 6.3.2, all
   implemented).
2. `ir::Type` is an alias of `ast::Type` ⇒ changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** in `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction in which behavior changes, and it is a fix rather than a
   regression**, but it must be confirmed by the third-layer corpus differential in 07.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the true `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the true `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match is sufficient) |

> **Design judgment**: because all 8 variants currently collapse to `Void`, **every code path that
> depends on signature bytecode** (the interpreter’s arity/type checks, serialization format)
> currently sees `Void` for all of them. After convergence, these locations will see the true types.
> **This is the document’s only potential source of behavioral change, and its direction is
> “restoring correct information.”** If the corpus differential finds a difference, it should be
> treated as **exposing an existing defect**, not as a regression introduced by this document. The
> method for adjudication is to construct a `.yx` program containing a `Ref` parameter and compare
> `dump_bytecode` before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                             | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Category                          |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`           | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**completed: WBS 6.3.1, 2026-10-29**—all 13 variants were deleted, and `ast::Type` was consolidated to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                        | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original variant position, **deleted**) | Delete `Expr::FnDef` (**completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consuming arms were deleted in the same batch, leaving zero references in `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                        | Delete `Assign.signature_params` (**completed: WBS 6.5.2, 2026-10-30**—the same-named field was also deleted from `TypeDefinition`, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm position, **deleted**)      | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Follows #3 (completed)            |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                        | Change the `NamedParen` comment to “syntax provides the binder name; semantic interpretation belongs to the **typechecking layer**” (**completed: WBS 6.5.2, 2026-10-30**—the comment became “parameter names belong to `Type::Fn.params`; this node only carries the return-position binder”; the `sema/` in the first draft is D1’s target directory name, while the current repository uses `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                                  | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | New                               |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                              | Add `trait TypeEnvProbe` + `NullProbe` (**completed: WBS 6.4.1, 2026-10-29**—the landed state also includes `BuiltinProbe` and `NameKind`; the trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | New                               |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                          | Add the `probe` field and accessor (**completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, and the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites were wired up**: 4 production + 36 test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                        | Use `state.probe()` to determine `NameKind` (**not done**: 6.4 introduced only the **classification enum + probe queries** and did **not** change the structure of `Type::Name`—fieldization would affect every production/test construction site and require the reverse bridge `mono_to_ast_type` to write back `kind` faithfully, forcing regeneration of IR snapshots and corpus baselines, which conflicts with this batch’s C3 zero-drift surface; fieldizing `NameKind` is deferred until a second consumer appears)                                                                                                                                                                                                                                                                                                                                                                           | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                        | Update the comment (`Result`/`Option` use `Generic`, already consistent with the facts; only add the variant-deletion explanation)—**not done**: 6.4 did not change `types.rs` (the change surface in that file belongs to the 6.3 fallback refactoring), and this line is deferred together with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                            | Delete the `_ => (Vec::new(), Type::Void)` fallback and use an explicit error return—**WBS 6.3.1 changed the fallback to a let-else that explicitly returns `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical), but **the “change to an explicit error return” half was not done**—the original plan deferred it to 6.4’s error-path refactoring, but **6.4 did not take over that path** (6.4 only handled injection of type-name dependencies; `types.rs` was unchanged) ⇒ **6.5 did not take it over either** (that batch only reshaped representations and left the fallback path unchanged) ⇒ **it moved to 6.6** and was registered in the 6.6.1 row of 09. The path is syntactically reachable (the `Name` arm of `semantic_tokens` records one additional dummy-span token; the corpus has 0 matches, so there is no drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                        | Delete `n == "Terminates"` and use `state.probe().is_predicate(n)` (**completed: WBS 6.4.3, 2026-10-29**—the landed state reads `NameKind::Predicate` through `ParserState::kind_of`: predicates declared in the current pass take priority, and the remainder is delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                        | Delete the `typecheck::operator_interfaces::spec()` call (**completed: WBS 6.4.3, 2026-10-29**—changed to `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                            | Delete the `signature_params` argument—**completed (WBS 6.5.2, 2026-10-30)**: all construction sites deleted the field; in the same batch, the HashMap name table in the name-merging section was changed to positional alignment (the criterion: prior validation guarantees element-for-element name identity)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                             | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Follows #3 (completed)            |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                            | Retain (the parameter-list retrieval point)—**“change to read Lambda directly as in 5.5” was not done**: it was not included in 6.5.1’s change list, and its disposition is to be decided together with 6.8.2’s declaration-driven checking mode                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`           | Delete the 13 match arms (one for each deleted variant) (**completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                        | Remove synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Consolidation                     |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                            | Retain the `DateTime` alias                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                              | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | New                               |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                                    | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Consolidation                     |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`)        | Delete `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                            | Keep `FunctionCode.params` as `Vec<MonoType>` (already so)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                     | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Consolidation                     |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                            | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Consolidation                     |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                           | Change the `type_table` conversion to `.collect()` (the elements now have the same type)—the actual change was to **delete the conversion and transport the values unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)               | **Delete** the entire `From<MonoType> for IrType` impl (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                             | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Consolidation                     |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                            | Handle the `AstType::AssocType` arms according to the gate conclusion—**the conclusion was determined and implemented: WBS 6.3.2 deleted all 5 variants / 10 sites registered in 03 §2.2.2 (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each)**, and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                     | Type-table gates T1/T2 (the original T3 was deleted following the 2026-10-30 user decision)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                            | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                           | Same, but obtain the name from `Assign.target`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                                   | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                                    | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                        | Delete the `Expr::FnDef` test (**completed: WBS 6.5.1, 2026-10-30**—the case was not deleted; it was rewritten as two Lambda-destructuring cases plus one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                        | Rewrite as Lambda + Assign (**completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`                  | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                        | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be processed in the same batch as #1**—**the criterion was corrected (completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`); after 6.1.2 deletes the lowercase spellings, it would become a `TypeRef` and **silently lose type identity**. The actual implementation therefore uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                                                                  | Consolidation                     |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                     | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` (**completed: WBS 6.4.3, 2026-10-29**—the landed signature is at `ast.rs:927`; the 3 production callers in `ir_gen.rs` pass `SemanticProbe`, and the others pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                          | Delete this file’s `name_used_as_type` and call the consolidated implementation in `ast.rs`; the `is_predicate` short-circuit reads `NameKind::Predicate` (**completed: WBS 6.4.4, 2026-10-29**—the consolidated single implementation is at `ast.rs:848` and covers the header decision + predicate short-circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                            | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3) (**completed: WBS 6.4.5, 2026-10-29**—dual-knowledge wiring: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe without the built-in-name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Consolidation                     |

> **The 6 completed rows of 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed
> 2026-10-28) implemented these 6 rows in one batch—four carriers changed to carry `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy `From<MonoType> for IrType` bridge
> and `ir::Type` re-export alias were deleted, and decoding was changed to transport values
> unchanged (`bytecode.rs:2315`, `:2316`, `:2346`). **The line numbers for these rows were refreshed
> to the landed state** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390` were
> pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured figures and gate
> results.

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, requires `mod`
> declaration wiring (this repository has already learned the lesson of 8 orphaned test trees). When
> the new crate is created, register it in the root `Cargo.toml` at the same time.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the 293 single-file `.yx` corpus files in `tests/yaoxiang/` + the multi-file corpus
layer established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library
**must all retain identical behavior** (the same diagnostic codes and messages, with ordering
normalized as needed).

| Risk                                                           | Assessment                                                                                                                                                                                                                                                                                     | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source contains type annotations such as `i64` / `int` / `f64` | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                              | **Measured** (end of 5.3): 0 noncanonical names in real type positions ⇒ 6.1.2 directly deletes the 22 spellings and **does not establish a lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Source contains `DateTime`                                     | Only the uppercase temporary compatibility alias is retained (`mono.rs:883`); it is not a built-in name                                                                                                                                                                                        | Moving it out of the built-in-name table belongs to P10 S11 (issue #513); until then, `DateTime` remains synonymous with `Int`, with no behavioral change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `("()")` occurs in type position                               | It is not a product of the `Name` form, but the type spelling of `Void` (standard-library signature strings write `() -> Void`)                                                                                                                                                                | **Retain**: `()` shares the `Void` arm and is one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `dump_bytecode` output changes                                 | The `type_table` element type changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                       | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only in the C1/C2 phases). However, the output diff must be recorded in phase 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Behavior changes after the lossy-bridge fix                    | See 6.2                                                                                                                                                                                                                                                                                        | Corpus differential + focused `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `mono_to_ast_type` rewrite changes type-substitution results   | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, the `_` arm would change a result such as `AstType::Int(64)` to `AstType::Name { name: "i64" }` | Phase 3 focused comparison: select corpus files with generic substitutions and compare the `type_name()` of monomorphization products before and after the change. **Do not assume `{:?}` output is equivalent.** **The criterion was corrected (phase 3 measurement)**: equality must not be judged using `type_name()`—it is the lowercase internal name used for diagnostic display, so comparing `type_name()` can only show that both are “`int64`,” not whether the canonical name is `Int64` or `i64`. Instead, judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` are mutual inverses for every name) + structural comparison of the write-back product. WBS 6.3 measured byte-for-byte-identical dumps and `.42` product md5 values for 4 generic-substitution programs |
| `.yx` source itself                                            | **No modifications**                                                                                                                                                                                                                                                                           | This document does not require changes to any corpus or standard-library source file. This is a hard acceptance item                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

**Hard acceptance conditions** (**criterion revised on 2026-10-27, carrying forward the 2026-10-26
decision ②**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly
driven test files** under `tests/` may be modified when mechanically required by the change, but
each such file must be registered with its **file / line count / rationale**, and **tests must not
be deleted or assertions weakened**. The original criterion (`git diff --stat tests/ src/std/`)
imposed a blanket zero-diff requirement on all of `tests/`; this revision relaxes the requirement by
one level for directly driven test files only, while preserving the zero-diff requirements for the
corpus layer (`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library
(`src/std/`). **Exception**: the 6.8.2 behavior-fix batch is handled separately and adjudicated
separately.

## Implementation Notes

The criterion uniformly used here is 07’s **C3: the same diagnostic codes and messages (ordering may
be normalized)**. The following is the phase breakdown for this workstream; see RFC-039 for the
global cross-document ordering.

| Phase                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Rollback point                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2 (T3 landed in phase 0 and was later deleted following the 2026-10-30 user decision). The first version of T1 **reports only and does not fail**. Establish a full baseline against **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | The T1 report accurately lists the 13 **parser-forward zero-construction** variants (including the classification of `AssocType`: forward 0 / AST→AST 2, D8 unchanged), and **separately lists the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; the 13 parser-forward zero-construction variants and the 1 globally zero-construction variant (`Enum` only) are reported separately; T2 is green or provides a repair list | No code changes; no rollback required                                                                                                                                                                                                                                                                                                                                                                                                     |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) was consolidated to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; the 7 lowercase duplicates in `add_builtin_types`, the `Uint` / `Int128` entries in the LSP list, and the `Uint` arm in `sizeof` were cleaned up in the same batch; **no lexer alias table was established** (corpus measurements found 0 noncanonical names). **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**verified: it uses the unfiltered original signature list; after 6.5.2 it is provided by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                                     | C3: the full-corpus diagnostic set is **identical entry by entry** to the phase 0 baseline                                                                                                                                                                                                                                                                                                                                                                                    | Separate commit; can be reverted                                                                                                                                                                                                                                                                                                                                                                                                          |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers changed to carry `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy `From<MonoType> for IrType` bridge (originally `bytecode.rs:2352-2390`) and `ir::Type` re-export alias were deleted (the `pub use` line at the head of `ir.rs` was reduced to a private `use`; that private `use` was also deleted in WBS 6.6.12); decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), so the previous silent information loss in which 8 classes of `MonoType` variants collapsed to `Void` no longer occurs                                                                                                                                                                                                                    | C3 + focused `dump_bytecode` comparison (each item in the 6.2 table)—**measured fully green** (transcript in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                             | Separate commit; can be reverted                                                                                                                                                                                                                                                                                                                                                                                                          |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: delete the 13 variants in the 5.2 table (`ast::Type` 26 → 13); remove the match arms at `mono.rs:699-708` and elsewhere, and the `types.rs:836` fallback (changed to a let-else that explicitly returns `Type::Name{"()"}`); **process `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; synchronize `types/tests/mono.rs`. **The criterion for #47 was corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” became invalid after 6.1 because `type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes lowercase spellings it would become a `TypeRef` and **silently lose type identity**. The actual implementation uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back | C3 + a fully green `cargo build` (exhaustive matches force completion of all arms) + focused generic-substitution comparison (last row of 6.4)—**measured fully green** (transcript in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                   | Separate commit; can be reverted                                                                                                                                                                                                                                                                                                                                                                                                          |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: add `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); change `parser_state.rs` (add the `probe` field and `probe()` / `kind_of()` accessors, **wiring all 40 construction sites** = 4 production + 36 test); delete the 2 `operator_interfaces::spec()` calls on the parser side, and change the 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); consolidate the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connect `CONST_PARAM_TYPES` through the dual `NameKind::Builtin` knowledge (#50). **Not done**: adding a `kind` field to `Type::Name` at `types.rs:162-165` (#10; fieldization conflicts with this batch’s C3 surface), and changing the `types.rs:836` fallback to an explicit error return (#12, moved to 6.5)                                        | C3 + **new assertions**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps (`typecheck` / `operator_interfaces`) measured 0 matches**; corpus 356/356, IR snapshots 216 with zero drift, `cargo test --all` exit 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                         | Separate commit; can be reverted                                                                                                                                                                                                                                                                                                                                                                                                          |
| **5. Dead AST variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: delete `Expr::FnDef` (baseline measured **25 sites / 16 files** = 1 definition + 15 production consumption sites + 3 test construction sites + 6 comments, of which 19 were code references) + delete the parallel parameter tables in `Assign` / `TypeDefinition`; reshape `Type::Fn.params` to `Vec<Param>` (names and types co-reside) → the single flattening entry point `Type::fn_params_flat`; retain `NamedParen` as a syntax node, with its binder interpreted explicitly by the type checker                                                                                                                                                                                                                                                                                                                                                           | C3 + all **22 modules** under `tests/integration/` green (22 `.rs` files measured; the original count of 18 was stale); formatter parameter-position output byte-for-byte unchanged (355 `.yx` corpus files compared byte-for-byte); corpus 356/356, IR snapshots 216 with zero drift; `#[test]` count in `src/` 2878 → 2883 (no tests deleted)                                                                                                                               | Split into 7 commits (2 variant-deletion commits + 3 representation-reshaping/old-representation-deletion commits + 2 fix-round commits); **revert criterion**: the variant-deletion commit can be reverted independently, while the test-migration commit must be reverted together with it (see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5); see [09a](./09a-wbs-history.md) §P6 6.5 for measured figures and gate results |
| **6. Gates become hard failures**             | **Completed (6.6.2, 2026-10-30 user decision 2A)**: T1 changed from warning to `panic!`, and **T1.6 survey-baseline reconciliation was connected to the same failure path** (a construction-site count differing from `SCOUT_BASELINE` fails the build); under decision 1A, `without_fn_param_names` was assigned to the second category (returning the forward count to its frozen value); T4 was downgraded to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | `cargo build` **must fail** when a zero-construction variant is deliberately introduced; changing any variant count in `SCOUT_BASELINE` by 1 **must also fail**—both mutations were tested using mechanically reversible patches (`scripts/dev/scratch.py trial`)                                                                                                                                                                                                             | Separate commit                                                                                                                                                                                                                                                                                                                                                                                                                           |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow phase 2**—before phase
2 lands, reverse bridge A on the bytecode side (`bytecode.rs`’s `From<MonoType> for IrType`)
consumes `IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation;
that bridge was deleted in 6.2.2. **The remaining change surface when phase 3 deletes the variants
is reverse bridge B** (`middle/passes/mono/function.rs`’s `mono_to_ast_type`, which still decodes
`String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`. Phase 3’s #47
must be processed in the same batch as #1; otherwise, `mono_to_ast_type` silently changes behavior.
**Phase 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B was changed to use
`canonical_name` + structural write-back; the three `ast::Type` carriers in `middle/core/ir.rs`
**were not changed**—6.3 only deleted variants and did not alter the IR shape (216 IR snapshots and
the 356-line corpus baseline had zero drift), and carrier disposition **was actually completed in
6.6.11 / 6.6.12** (two zero-producer dead variants were deleted, `TypeDecl` was replaced with an
IR-native narrowed carrier, and `ir.rs` no longer imports `ast`); the original “defer to phase 4”
schedule was not fulfilled, and this paragraph governs |

**Dependency on downstream work**: only after all phases in this document are complete may
[04-ssa.md](04-ssa.md) begin. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all have the same element type, giving SSA type annotations a single landing point.

### Mandatory Validation for Each Phase

| Phase | Must execute                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, and manually verify the T1 report (**including separately listed reverse-bridge reconstruction sites**)                                                                                                                                                                                                                                                                                                                     |
| 1-5   | `cargo test`; full-corpus differential over `tests/yaoxiang/` (07 third layer); entry-by-entry comparison under the C3 criterion                                                                                                                                                                                                                                                                                                                                                                  |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                                |
| 3     | Additionally: for corpus files with generic substitutions, compare the `type_name()` of monomorphization products before and after the change—**the criterion was corrected (phase 3 measurement)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient for equality; judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` are mutual inverses for every name) + structural comparison of the write-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                                   |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                       | Rationale                                                                                                                                                                                                                                                                                                        |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                                                                                                                                        | The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) is the direct cause of the mismatch between the `BytecodeFunction.params` and `FunctionCode.params` field types (2.1). Once the alias is deleted, both field types are unified and the `From` bridge loses its reason for existing |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                      | It would create an L2→L3 reverse dependency (violating RFC-039 layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See 5.1 for the full argument                                                                                                                                               |
| Do not delete `NamedParen`   | Retain it as a pure syntax node; migrate binder semantics to the **typechecking layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is D1’s target directory name, while the current repository directory is `typecheck/`) | `ast.rs:518-532` documents that deleting it would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as the binder                                                                                                                                        |
| Directory renaming           | **Yes** (D1), performed as a pure-move batch at the end of P5/P6; the body of this document uses current pre-rename paths                                                                                                                        | 2026-10-03 decision (ChenXu233); 2026-10-04 supplement on construction method                                                                                                                                                                                                                                    |

### Rejected Alternatives

None of the four alternatives was adopted: **retaining all three representations and adding only a
conversion layer** resolves none of the root causes, and `ir::Type` remains a non-independent type,
so none of the dead variants, synonym table, or reverse dependencies disappears; **using a macro to
generate the synonym table** treats a symptom rather than the cause—once `Type::Int(usize)` is
deleted, the macro has no reason to exist, and consistency is instead guaranteed by the T2 gate
(more verifiable than macro expansion); **deleting only dead variants without unifying the
representation** has the lowest cost but leaves the problem in place, with no gate preventing the
next person from adding another `Type::Int128(usize)` and recreating the same dead variant (if only
one commit is possible, it should explicitly be defined as a subset of this alternative and include
delivery of the T1 gate); **unifying on `MonoType`** is discussed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. The 15 production consumers of
  `Expr::FnDef` (including 3 exhaustive arms) + 3 test construction sites + 1 variant definition
  form a never-executed parallel path; deleting it removes one branch from each of `ir_gen.rs` /
  `ownership.rs` / `termination.rs`.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  already differ in definition-body field recursion; after merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legitimate type value.”** This
  is the prerequisite for 07’s first-layer validator’s “type consistency” invariant to pass.
- **Gates make regression impossible**. T1 turns “adding a zero-construction variant” from silent
  passage into a build failure—the existing `build.rs` mechanism in this repository proves that this
  approach works.
- **Zero source-code changes**. The 293 `.yx` corpus files and the standard library require no
  modifications, which is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may change behavior**. After the lossy bridge is fixed, the 8 `MonoType` variants can
  now be seen with their true types rather than `Void`. The direction is a fix, but it changes paths
  that depend on signature bytecode. This is the document’s largest uncertainty.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify. This is why phase 0 requires manual
  verification.
- **If T1 fails to count reverse bridges, the disposition table and gate contradict each other**.
  2.2 proves that 7 “dead variants” have production reconstruction sites in reverse bridges. If T1
  does not classify construction-site direction separately, those 7 are classified as “having
  construction sites” and exempted from deletion—the gate would become an obstacle to the deletion
  work rather than a safeguard. | **2. Deleting `signature_params` carries semantic risk**
  (**verified and resolved: 6.5.2, 2026-10-30**). It carries the **unfiltered** parameter list,
  whereas `Lambda.params` contains the **filtered** list. If `classify_generic_params` depends on
  the former, the refactoring would change generic-classification results. **Measured conclusion**:
  `classify_generic_params` / `extract_generic_param_names` need the unfiltered list; flattening
  `Type::Fn.params` and the old `signature_params` come from the same source (the same `Param`
  nodes), so classification does not change. The proposal to use `Lambda.params` would change
  behavior and **was not adopted** (corpus 356/356, snapshots 216, `cargo test --all` fully green).
  | **3. Phase 3 touches `middle/passes/mono/function.rs`**. Besides the 8 `AssocType` matches,
  `mono_to_ast_type` at `507-529` is the exit point for monomorphized type substitution, and
  rewriting it may change the substitution result’s `type_name()` (last row of 6.4).
- **The `.yx` corpus may contain noncanonical type names**. If many files use `i64` / `int`, phase
  1’s “directly delete synonyms” approach is infeasible and must be downgraded to “move them into a
  lexer alias table,” reducing the scope of `from_builtin_name` consolidation. (**Measured as 0**:
  the 6.0 survey is recorded at the end of 5.3, so WBS 6.1.2 directly deleted the aliases and did
  not take the alias-table route.)
- **The gate may initially be red** (**this risk was handled in two steps**). If T1 launched
  directly with `panic!`, it would immediately block compilation; therefore, from 6.0.1 it first
  reported only and established a baseline, and only on 6.6.2 did it switch to `panic!`. By the time
  it became hard, the baseline already matched all 13 rows under the same criterion, and the current
  tree has 0 zero-construction variants.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **No unresolved items remain in this document.**

## See Also

### Upper-Level and Peer Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Charter)](../../rfc/accepted/039-compiler-architecture.md)
  — overarching charter; four-layer model, criterion tiers, routing table C (3
  type-representation-related entries assigned to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction rules, target
  directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document’s prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; operator change-surface
  consolidation
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, type-consistency invariant for the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  `Type::Generic` unified path
- [RFC-011 Generics Type System](../../rfc/accepted/011-generic-type-system.md) — generics mechanism
  and type-level constructors (`If` / `Match`); **does not carry a built-in type-name list**, so it
  is not a gate-comparison target (the original T3 was deleted following the 2026-10-30 user
  decision; see 5.7)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of a build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line-number anchors were refreshed to the 6.5 landed state** (2026-10-30); entries removed by
> preceding batches are retained and labeled with the deletion batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after the 6.3
  closeout; the 26-variant era spanned 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — explanation of `Type::NamedParen` binder semantics
  (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types co-reside) and
  `Type::fn_params_flat` (flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- (**Deleted in WBS 6.6.12**) `src/frontend/core/parser/ast.rs`’s `Type::without_fn_param_names`: a
  printing-boundary consolidation introduced in the 6.5 batch for “using `ast::Type` as the IR
  carrier”; after 6.6.12 replaced `FunctionBody::TypeDecl` with an IR-native narrowed carrier, the
  AST no longer enters the IR, the function lost its sole production call site, and it was deleted
  (T1’s `REBUILD_FUNCTIONS` / `SCOUT_BASELINE_CALIBER` lost one entry in the same batch)
- `src/frontend/core/typecheck/probe.rs` (from WBS 6.6.9) — `CONST_PARAM_TYPES`: the sole
  authoritative list of **acceptable type names for const generic parameters** (originally in
  `parser/ast.rs` as the third copy of L3 knowledge; after the parser’s closed set was cleared, it
  moved back to L3, and the parser asks only through `TypeEnvProbe::is_const_param_type`; see 5.4)
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single implementation after the
  6.4 consolidation (the original `name_used_as_type_in` and the copy in `declarations.rs` were
  merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through the injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: the fallback for
  non-`Fn` forms (returns `Type::Name{"()"}` since 6.3; the `Type::Void` variant was deleted)
- (**Deleted in 6.4**) `src/frontend/core/parser/statements/declarations.rs:42-80` — the original
  `name_used_as_type`, merged into the single implementation in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — the hardcoded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call (changed in 6.4 to use `ParserState::kind_of` / the
  injected probe; both literal references have disappeared, leaving only a comment)
- `src/frontend/core/parser/pratt/nud.rs:512` — expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — the `expr_to_params` arm retrieves the parameter
  list
- `src/frontend/core/types/mono.rs:863` — the `from_builtin_name` built-in-name parsing table (15
  names + the temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:960` — the `From<ast::Type> for MonoType` implementation (12 of
  the 13 match arms are here)
- `src/frontend/core/types/mono.rs:169` — `MonoType` definition (approximately 22 variants)
- `src/frontend/core/types/const_data.rs:240`, `:327` — `const_data::BinOp` / `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- (**Deleted in 6.2.2**) `src/middle/core/ir.rs:3` — evidence that `ir::Type` was an alias of
  `ast::Type`
- `src/middle/core/bytecode.rs:809` (`BytecodeFunction`), `:814` / `:816` (`params` / `return_type`,
  carrying `MonoType` since 6.2)
- (**Deleted in 6.2.2**) `src/middle/core/bytecode.rs:2352-2390` — lossy `From<MonoType> for IrType`
  bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `:275-278` — `BytecodeFile::type_table` and the
  `Vec<MonoType>` field of `FunctionCode`
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: the reverse bridge after the 6.3
  rewrite (using `canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — the interpreter module’s `type_table`
- `build.rs:33-51` — build-time gate for the error-code registry (a model for the type-table gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
