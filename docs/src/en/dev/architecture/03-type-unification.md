# Type Representation Unification

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, tiered acceptance criteria, and execution-stage
> ordering; see the [index for this directory](index.md) for the positioning of all companion
> documents.

## Positioning and Scope

### Coverage

Consolidate the current parallel type representations into one, and eliminate the four categories of
burden derived from them:

- **13 `ast::Type` variants with zero parser-forward construction sites** (among the 26 variants in
  `ast.rs:427-541`, see 2.2; the “forward” count excludes reverse bridges and AST→AST reconstruction
  points)
- **A handwritten table of type-name synonyms** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser's genuine reverse dependency on the typecheck layer** (2 sites, see 2.4)
- **One expression variant with zero production construction sites**, `Expr::FnDef` (**removed: WBS
  6.5.1, 2026-10-30**—the variant definition and every consumption arm were deleted; see 2.6)

The concrete deliverables are four items: a per-variant disposition table for all 26 variants (5.2),
semantic classification of `Type::Name` through `NameKind` and elimination of the synonym table
(5.3), an injected `TypeEnvProbe` data flow (5.4), and build-time type-table gates T1–T3 (5.7).

### Directory Renaming and the Path of This Document

RFC-039 Decision D1 mandates renaming the directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) **as part of P5/P6, without splitting the work into stages**. This
document's 50-item change list and all acceptance greps are written against the **pre-rename
paths**—they are the construction baseline for the P6 batches **before the rename batch**. The
directory rename at the end of each batch is a **pure move batch** (C1 zero-diff) and is committed
separately. The baseline becoming invalid after the rename commit is expected; use git history as
the bisection reference.

### Not Covered

- **How to execute the directory rename**. `Type` is not moved out of `parser/ast.rs` in this stage;
  only `lower.rs` is added under `types/` as the sole conversion point. The directory rename
  (`typecheck/`→`sema/`, etc.) is performed with P5/P6 closure according to RFC-039 Decision D1, in
  the form described by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and validator implementation**. The definitions of C1–C6 and the
  three tiers of criteria (IR validator / normalized snapshots / corpus differential testing) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document cites only **C3
  (type-representation convergence)** from that set as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite; the reason is
  given below.
- **The global P1–P10 execution sequence**. See the main body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document gives only the stage breakdown for its
  own workstream (see “Implementation Essentials”).

### Division of Responsibility with RFC-039

RFC-039 is the higher-level master plan: the four-layer model, criterion tiers, cross-document stage
ordering, and the rationale for performing the refactoring now. This document is the construction
plan for **type-representation convergence**, one of RFC-039's workstreams: exactly which variants
to delete, which files to modify, in what order, and which gates prevent regressions.

Every line number, variant list, and construction-site count in this document comes from static
verification of `src/`; each item provides its `file:line` evidence. **This document does not record
the execution process; it records code facts and the resulting dispositions.**

For ease of cross-referencing with documents in the same batch, the “Target Design” and “Detailed
Design” chapters retain their `5.x` / `6.x` subsection numbering; all other chapters follow this
directory's unified conventions.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type; it is a `pub use` alias of `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) will introduce register type annotations and new instructions into the IR. If
representation is not consolidated beforehand, SSA type annotations will grow directly on the
26-variant syntactic type through the `ir::Type` alias, effectively institutionalizing a third
representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree inside a type.
  It is impossible for an `Expr` to appear in an IR instruction's type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and `Span`. This
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently fossilize the 13 dead
variants and the two syntax-only variants in the IR contract.

## Current State

Everything below is a statement of code fact, not a proposal. Each item provides `file:line`
evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 landed)**: This section records the state **before** 6.2
> landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed four type carriers to carry `MonoType`, removed
> the lossy bridge `From<MonoType> for IrType`, and removed the `ir::Type` re-export alias; every
> statement below involving `ir::Type` or the lossy bridge is a **pre-change** fact, and statements
> marked “removed in 6.2.2” describe an effective disposition. See [09a](./09a-wbs-history.md) §P6
> 6.2 for implementation details and measured figures.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                               | Causal relationship to type representation                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, the parser selects the latter, and typecheck must add a table to translate the former back                                                                                       |
| Bytecode-layer signatures differ from IR-layer signatures                        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one built-in type name requires changes in multiple places without checks | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; both places duplicate it without cross-validation                                                                                                                                          |
| The parser hard-codes `"Terminates"`                                             | Because the parser cannot determine “is this a type?” or “is this an operator interface?”, it can only embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                            |

**The first three are representation problems; the fourth is a downstream side effect of a
representation problem.** Addressing only item 1 (deleting dead variants) would leave the synonym
table and reverse dependency in place; addressing only item 4 (injecting a callback into the parser)
would build on top of a still-inconsistent representation.

One of the three parallel representations is an alias illusion:

| Name        | Definition location                       | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, followed by `ir.rs:6`, which uses
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two “type” symbols
simultaneously, but one is merely an alias.

There is a fourth type carrier on the serialization side:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was removed in 6.2.2).** `From<MonoType> for IrType`
(previously at `src/middle/core/bytecode.rs:2353-2390`) degraded `MonoType` into `ast::Type`. Its
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

All eight `MonoType` variants collapsed into `IrType::Void`, in addition to the `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`); the comment itself admits that this was “a transitional branch
before deleting the enum variant.”

`IrType::Void` is **a legitimate type value here, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function's return type was lost
during bridging”—one reason why the first-layer “type consistency” invariant of 07 cannot currently
pass on this code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants targeted in 2.2—**both ends
of the lossy bridge point to dead code**.

**Current state (since 6.2.2)**: The impl has been **removed entirely**, and decoding no longer
degrades and reconstructs `MonoType` as `ast::Type`—all four carriers now carry `MonoType`:
`BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same file, `:816`),
`BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now transports the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those
eight `MonoType` variants retain their real types in signatures and type tables, and `IrType::Void`
is no longer used as a substitute for “information loss.” The `ir::Type` re-export alias
(`pub use ... ast::Type` at `src/middle/core/ir.rs:3`) was also downgraded to a private import
(`:7`)—the IR layer no longer publicly re-exports the syntax-layer type. However, the IR layer still
has three carriers that directly hold `ast::Type` (`Instruction::Cast.target_type` /
`Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`), pending disposition in 6.3.

### 2.2 Parser-Forward Variants with Zero Construction Sites

**Counting methodology**: The construction-site scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. **Construction site = an expression-position `Type::<variant>` path**; pattern
positions are not counted. Results are **listed in three separate categories and must not be
conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero construction sites in the
parser-forward direction** (the other two reconstruction categories are described in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                            |
| ------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                          |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                          |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                          |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                          |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an explicit exemption; eliminated with 6.4 #12)                                                                                                                                                    |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                          |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 AST→AST production reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Definition-column line-anchor drift registry (P6 6.0 measurement, 2026-10-27)**: The
> “Definition” column in this table and in the 5.2 table was written on 2026-10-05. Since then, 17
> lines were inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so **the
> definition lines of all 26 variants uniformly shifted by +17** (for example, `Int` moved from 432
> → 449, and `NamedParen` from 533-540 → 550-557). This round only **records** the drift and does
> not change the table body; subsequent references use the `ast::Type` symbol anchor.

The complete causal chain can be verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… they follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site of `Type::Void` is an error
   fallback**—`_ => (Vec::new(), Type::Void)` at `types.rs:836`, at the end of a match returning
   `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document uses the term “**zero parser-forward construction sites**”: it means
neither “zero references globally” nor “zero production construction sites”—11 of these variants do
have production construction sites in the other two categories. These tests themselves codify the
behavior under test: they assert exactly what these syntactic forms lower to as `MonoType`, even
though the parser never produces them. When deleting a variant, these tests must be deleted or
rewritten accordingly.

#### 2.2.1 Boundary Condition (1): The Reverse Bridge Reconstructs Seven of Them

The table's “zero forward construction sites” classification holds only in the **parser → AST
forward-construction** direction. In the opposite direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct seven of the variants in the
table**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant list                                                                             |
| -------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `From<MonoType> for IrType`                                    | `bytecode.rs:2353-2390`                  | `Int` (`2357`), `Float` (`2358`), `Bool` (`2359`), `Char` (`2360`), `Void` (`2361`), `String` (`2372`), `Bytes` (`2373`) |
| `mono_to_ast_type` (nested fn inside `substitute_type_in_ast`) | `middle/passes/mono/function.rs:507-529` | `Int` (`512`), `Float` (`513`), `Bool` (`514`), `Char` (`515`), `Void` (`516`), `String` (`522`), `Bytes` (`523`)        |

```
// middle/passes/mono/function.rs:511-527
match mono {
    MonoType::Int(n) => AstType::Int(*n),
    MonoType::Float(n) => AstType::Float(*n),
    MonoType::Bool => AstType::Bool,
    MonoType::Char => AstType::Char,
    MonoType::Void => AstType::Void,
    MonoType::TypeRef(name) => AstType::Name { name: name.clone(), span },
    // #299：String/Bytes 等容器类型现为 Generic，经 type_name 反解回 AstType
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This boundary directly affects disposition**:

- The `bytecode.rs` site disappears with 6.3's #28 (deleting the entire `From<MonoType> for IrType`
  impl), so it adds no cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a type-name reverse
  resolver (`MonoType → AstType`, annotated `#299` in the comment) that resolves `String` / `Bytes`
  from `Generic` back into dedicated variants by name. After deleting `ast::Type::String` / `Bytes`,
  these two guard branches must be rewritten; after deleting `Int` / `Float` / `Bool` / `Char` /
  `Void`, behavior of the `_` arm changes (some types that previously fell into `_` now silently
  take the `AstType::Name` path). This is supplemental item 47 in 6.3.
- **Direct consequence for the gates**: A T1 implementation that scans only forward construction
  sites would classify the seven variants above as having construction sites and thus exclude them
  from the removable list. T1 must explicitly include reverse bridges in its counting scope;
  otherwise, its conclusions contradict this document's disposition table (see 5.7).

#### 2.2.2 Boundary Condition (2): AST→AST Reconstruction (the third category added in this revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` are **AST→AST** type-variable substitutions (they rebuild an
existing AST type; they are **not** `MonoType → AST`). A P6 T1 gate measurement (6.0.2 baseline,
2026-10-27) found **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to the
13-variant list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these five variants, it must rewrite these ten sites in the same batch** (09's
6.3.2 registers them); otherwise, these arms will fail to compile after the variants are deleted.
**Implemented (WBS 6.3.2, 2026-10-29)**: all ten sites were deleted; see the disposition table in
[09a](./09a-wbs-history.md) §P6 6.3.

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

**The sole reason this table exists is to bridge inconsistency in the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but structurally
different at the AST layer, and the parser produces only the latter, `from_builtin_name` must accept
case variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"` on the
`Void` line is especially revealing—it accommodates the `Type::Void` fallback at `types.rs:836`.

The same knowledge of “which names are built-in type names” exists in seven copies throughout the
repository, with no consistency validation whatsoever:

| #   | Location (**measured 2026-10-27**; this section was originally written on 2026-10-05, and the original line numbers have drifted) | Form                                                                                                                                                                                                                     |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                          |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, duplicating `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                             |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                 |
| 4   | `src/lsp/world.rs:182`                                                                                                            | Built-in type-name list on the LSP side (14 names = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                  |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Coverage differs from item 1**: it excludes lowercase and the `i64` / `i32` alias families (P0 gate registry, 2026-10-06)                                                            |
| 6   | Base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                           | String match, name → byte size. Previously included a `"Uint"` arm (item 1 had no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports Unknown type. **Dead arm and live evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typecheck side's “Set is excluded by name” decision (`inference/expressions.rs:2917-2918`, #300 Decision 4)                              |

> **Line-anchor drift registry (P6 6.0 measurement, 2026-10-27)**: The line numbers for all seven
> copies in this section **have drifted** (item 1 by +223; the others by 2–9 lines each), but **the
> entry counts and contents have not changed** (item 1 had 13 arms when this section was written,
> item 3 had 15 entries, and item 4 had 15 entries when this section was written; after WBS 6.1.2,
> item 1 has 15 arms / 16 spellings and item 4 has 14 entries). Subsequent references use the
> **symbol anchor + repository path**; the line numbers in this table are for location only. **All
> remain**: item 1's arm count and the drift evidence from items 5 / 6 / 7 (the dead `Uint` arm and
> the contradiction between `Set` and the typecheck side). This section did not change code.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to identify const generic parameters. It
enumerates “type names that may serve as const parameters,” which has a **partially overlapping but
non-identical relationship** with the full set in `from_builtin_name`, and no mechanism guarantees
synchronization. Item 4 is a fourth copy used for LSP syntax highlighting and hover, also without
validation; T2's comparison scope includes it.

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 / 7 share the same root as items 1–4
(string-key tables mapping built-in names to semantics / IDs), although their uses differ, they
carry the same drift risk. The same gate also found 11 built-in container-name string dispatches on
the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4, etc.).
**These are outside both this section and the parser data-flow scope of 5.4**; they are listed and
dispositioned in [06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 under the P6
extended-verification items.

### 2.4 The Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both of which supposedly belong to the type
layer). **Measurement differs from that description**: both functions are defined inside the parser,
not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository's only `fn`
  definition)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the repository's only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production-code sites** that cross layer boundaries and reference `typecheck`;
both call the same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                 | Purpose                                                                                         |
| --- | --------------------------------------------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Identifies a constraint-position parameter (`T: Add`) and excludes it from runtime parameters   |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Identifies a constraint-position parameter and produces a `GenericParamName` with `constraints` |

The second site is deeper—**`parser/ast.rs` itself** (the entire `ast.rs:847-948` block, which
shares a file with `StmtKind` / `Expr`) directly references a typecheck symbol. Routing table C
records only the `declarations.rs` site.

Together with the hard-coding at `declarations.rs:498-499`, it forms the same judgment:

```rust
// declarations.rs:498-499
// Extended as more built-in predicates are added (the future open-set approach: pass predicate names down to
// the parser, or use a form decidable at the syntax layer).
// The parser does not hold a type environment, so it looks in two places: the hard-coded built-in predicates,
// plus declarations parsed in the current pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment itself acknowledges this is a temporary form of the “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem**: The parser needs to ask “is this an
operator-interface name?” because it must distinguish whether `Type::Name` refers to a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings** (concrete type / type variable / constraint name / predicate name), and the
syntax layer cannot decide on its own; the authority is therefore forced upward into the type layer.

#### Duplicate Implementation: Same Name and Meaning, but Behavior Has Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` acknowledges
this:

```rust
/// Whether the parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; this implementation is separate to avoid
/// a cross-module dependency inside the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` at `declarations.rs:42-80`                                  | `name_used_as_type_in` at `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                         | No callback                                          |
| `Type::Generic` branch  | First short-circuits on `is_predicate(app_name)` and returns `false` (`:54-56`) | No such short-circuit; directly recurses into `args` |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                            | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                      | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records why the latter two branches were added: “These two branches
were previously missing, so references to const generic parameters in a definition body could not be
identified as ‘used as a type’; annotation validation falsely reported a valid const parameter as an
unknown name.”

In other words: **the function was duplicated to avoid a cross-module dependency, but its fixes
never flowed back to the original**. The `ast.rs` version recursively handles fields in definition
bodies, while the `declarations.rs` version does not—the same source segment therefore produces
different results at two decision points. This duplicate must be merged when 5.3 introduces
`NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums. They do not reference each other, and **their members have already
diverged**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                     |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer uniquely has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer uniquely has `Deref` (`:222`); the constant layer uniquely has `BitNot` (`:329`)                                              |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at `const_data.rs:260-289`).

**Why this is part of the type-representation issue**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they appear in type-position expressions,
they fall into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly
related to the IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears
following `Type → MonoType` and is never visible to the IR, while `Assign` / `Deref` continue to
exist through paths such as `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)`
(`inference/expressions.rs:875`). The divergence between the two enums makes it impossible to
determine from either side alone which operators may appear in type positions.

This stage does not merge the two enums (their ownership is covered under “Changes to the Operator
Surface” in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate's scan scope should
include `ast::BinOp` / `ast::UnOp`, making newly introduced zero-construction operators equally
visible.

### 2.6 `Expr::FnDef`: Expression Variant with Zero Production Construction Sites (Removed: WBS 6.5.1, 2026-10-30)

**This section records an implemented fact**: `Expr::FnDef` used to be a variant of `ast::Expr`. The
claim of **zero production construction sites** is correct—the repository contained only two
construction sites (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`), both in tests.

However, **there were far more than four consumption sites**: the baseline measurement found **15
production consumption sites** (categorized in 5.5), plus six comments mentioning the variant; in
the 6.4 closure state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1
definition + 15 production consumption sites + 3 test construction sites + 6 comments.

**The path actually taken by function definitions** is: `pratt/nud.rs` constructs `Expr::Lambda`,
and `parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in
`value`). In other words, the `FnDef` branch was **a never-executed parallel path maintained at 19
code locations** (15 consumption sites + 3 test construction sites + 1 definition).

**Landed state**: The variant definition and every consumption arm have been **completely deleted**;
`git grep -n "Expr::FnDef" -- src/` now returns 0 matches within `src/` (the bare word `FnDef`
remains only in two explanatory comments). See [09a](./09a-wbs-history.md) §P6 6.5 for the per-site
disposition table, test migration, and measured figures; see 5.5 for the design of the merge.

### 2.7 Syntax Nodes Store Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` once carried typechecker-specific fields (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>` (formerly
`ast.rs:244-245`), and its comment explicitly stated: “The first parameter group of the signature is
preserved as-is (including parameter names) for the typechecker's `classify_generic_params` use”—a
statement AST reserved a dedicated slot for the type checker.

Current representation: Parameter names and types **co-reside** in the type annotation's
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` now carries the declaration
signature as `type_annotation: Option<Type>` (the same carrier as `Assign`).

**Second, expression position is used as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and uses it as a parameter list.

In other words: a normal AST variant is used as a **temporary parameter-list carrier**, and another
module's match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:518-532` explicitly states:

> RFC-027 §3's return-position refinement relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable in a constraint that is not in scope is the return-value formal parameter,” so
> `(r: P(m))` silently substitutes the undeclared `m` as well (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Claim: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is demoted to the type checker's internal working representation; the `ir::Type` alias is
deleted.**

The three representations converge into **two roles**, but only one is a “type”:

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

**Why not “merge into `MonoType`”** (this is the proposal most likely to be raised, and it is
rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), whereas
   `MonoType` does not. C3 in 07 requires identical diagnostic codes and messages—diagnostic
   positions must remain traceable to source. Adding `Span` to `MonoType` would burden monomorphized
   types with meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker's solving
   process**, not types themselves. Placing them in the sole type structure would require the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back.
4. **The decisive reverse argument**: Elevating `MonoType` to the sole representation makes the L2
   syntax layer depend on the L3 semantic layer's type. RFC-039 explicitly forbids L2 from depending
   on L3. Eliminating reverse dependencies is one of this document's objectives, so selecting a
   design that creates a reverse dependency would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side at `codegen/bytecode.rs:277-278`
uses `MonoType`. The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed entirely, also
eliminating the reverse reconstruction recorded in 2.2.

**Explicitly out of scope**: `Type` is not moved from `parser/ast.rs` to `sema/types/` in this
stage. Only `lower.rs` is added under `types/` as the sole conversion point, and the definition of
`Type` remains in place. The directory rename is performed as an independent move batch at P6
closure according to D1—at that point, `Type` belongs at the top-level AST domain in `ast/type_.rs`,
while `sema/types/` is the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> **Implemented (WBS 6.3, 2026-10-29)**: All 13 “Delete” items in this table **have been
> implemented**—`ast::Type` converged from 26 variants to 13 variants (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were **rewritten in the same batch**. See [09a](./09a-wbs-history.md) §P6 6.3 for
> measured figures, the 10-site disposition table, and acceptance criteria.

> Disposition categories: **Delete** (zero parser-forward construction sites), **Retain** (has
> forward construction sites), **Migrate** (its semantics move to another node).

> **Line-number convention**: The “Definition line” column gives each variant's **original location
> before deletion** (the 26-variant era; 13 were deleted in 6.3). The line numbers for matching
> sites in the “Rationale” column also use the **baseline from before 6.3** and are not updated as
> code shifts—this batch only refreshes the definition lines for variants that still exist (rows 14
> and 26) and corrects an obsolete consumption site in row 14.

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthened)                  | The parser's **sole** primitive-type exit (`types.rs:162-165`). After deleting the 12 primitive-type variants, every primitive type takes this path. It needs a “semantic kind of name” annotation (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:699` is a match arm). Bit width is carried by the `Name` path through `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it, and that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:703-706` is a match arm). `bytecode.rs:2373` likewise consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The sole parser-forward construction site = error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an explicit exemption). The fallback is eliminated with 6.4's error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumption sites (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction site `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:743-746` is a match arm); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:747` is a match arm); zero across all three categories (the only globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`, unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                                 | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering); the original `bytecode.rs:2366` referred to `IrType::Fn` and disappeared with 6.2's removal of the `ir::Type` alias and lossy bridge. **Field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types co-located) accompanies 6.8.1 (D54)**—the names formerly in `Assign.signature_params` are thereby carried here (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous`/`from_type`/`is_named`; anonymous parameters (pure type positions) are synthesized with empty names, so this table's “net effect” is unchanged; `Param` has a hand-written `Debug` implementation (anonymous parameters print according to their carried type ⇒ zero IR snapshot drift, see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) rewritten in the same batch as 6.3.2. `types.rs:392-396` already states that the `Generic` path is used                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero parser-forward construction sites (the syntax layer has no `::` associated-type path); **2 AST→AST production reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten with 6.3 (registered in 6.3.2). **D8 unchanged**: the criterion is forward-oriented; if associated-type syntax is enabled in the future, a new proposal will define it then                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero parser-forward construction sites (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction site `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type in unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has production construction site `types.rs:160`. It is the only `Type` variant that embeds an `Expr`; the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Its semantics (the binder name) move to the type checker; the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now says “parameter names belong to `Type::Fn.params`; this node carries only the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction must be addressed before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs seven of rows 2–8 in this
> table; in addition, the AST→AST reconstruction sites recorded in 2.2.2 include five of the 13
> variants across 10 sites (2 each for Union/Option/Result/Sum/AssocType). Both categories of
> reconstruction sites must be handled in the same batch as 6.3.1; otherwise, the result is either a
> silent behavior change (see 6.3 #47) or a compilation failure.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly according to
> D8 (5.2 table row 18). The **criterion is parser-forward**: zero forward construction sites is
> established, because the syntax layer has no path that produces a `::` associated type. Therefore,
> D8 need not be changed and the variant does not enter the allowlist. Its **2 AST→AST production
> reconstruction sites** (`function.rs:456`'s `substitute_type_ast` and `function.rs:623`'s
> `substitute_type_in_ast`) were registered in 6.3.2 and rewritten with 6.3. The three-category
> breakdown of the T1 gate (5.7) reproduced this conclusion (all 26 variants matched individually in
> the 6.0.1 measurement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` needs to determine “is this a
predicate?”, lines 510-512 of the same file need to determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` needs to determine “is this an operator interface?”, and the
two implementations of `name_used_as_type*`—`name_used_as_type_in` in the same file (measured
at 864) and `name_used_as_type` at `declarations.rs:42`—need to determine “is this a type
reference?”, all through string comparison.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target shape (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type names: Int / Float / Bool / ... → delegate to from_builtin_name
    UserType,     // User-declared type names (including Generic's std.result / std.option)
    TypeVar,      // Type variables: T / K / V
    Constraint,   // Operator interfaces / constraint names: Add / Ord / ...
    Predicate,    // Compile-time predicates: Terminates / Sorted / ...
}
```

**Key design point: classifying `NameKind` requires exactly the information the parser currently
cannot obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be classified independently inside the parser**—it
must be classified by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 landed, 2026-10-27, final user decision)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) is **retained, with its
semantics demoted**. Before the refactoring, it was the disambiguator that bridged two equivalent
AST representations; after `Type::Int(64)` is deleted, `"Int"` has only one representation path, and
it becomes the **sole name-to-type resolution table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language built-in names (the
  signature strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP list all use it as their
  reference).
- The `DateTime` arm records a real fix (#338 collateral issue: the return value of `now()` could
  not be passed to `format_time`)—it is a **semantic alias**, not another case alias of `Int`/`int`,
  so it is retained separately until P10 S11 (issue #513).

**Resulting disposition** (final definition = 15 built-in names + temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                     | Before                                          | Disposition                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Mixed into 13 arms with aliases                 | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                       |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                        | **Deleted** (22 spellings), **with no lexer alias table**—corpus measurement found 0 non-canonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | Shared with `Void`                              | **Retained**—`()` is a spelling of `Void` (standard-library signature strings use `() -> Void`), not another type                                                                                           |
| `DateTime`                                                                                                                                                                                                                                   | `datetime` and lowercase aliases shared one arm | **Retain only the uppercase** temporary compatibility alias, annotated as migrating out under P10 S11 (issue #513)                                                                                          |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares an arm
with `Void`; there is also one temporary `DateTime` alias). After deleting the 22 spellings, a
non-canonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—it is **no longer silently accepted**.
Previously, `add_builtin_types` also registered seven lowercase copies (`int` / `float` / `bool` /
`string` / `void` / `never` / `char`) in the type table, effectively forming a second alias table;
these were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for the 22 spellings, and `DateTime` → `Int(64)`);
2. T2.5, the “Deleted Spelling Sentinel,” in `tools/type-tables --report`—if any of the 22 spellings
   is added back to the resolution table, the report immediately counts it as **unregistered drift**
   (there is no disposition to register: that would be a regression).

**Merging the two `name_used_as_type*` implementations**: The two duplicate implementations recorded
in 2.4 must be merged into one; otherwise, the classification information supplied by `NameKind`
will be bypassed by two independent string-comparison implementations. The merged implementation
belongs in `parser/ast.rs` (the data source is near `Type`), and `declarations.rs` calls it. The
`is_predicate` short-circuit reads `NameKind::Predicate`, while `Struct` / `NamedStruct` field
recursion uses the newer, more complete version at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: The const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps the full set in
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the check in
`extract_generic_param_names` (same file, measured 932) can become “`kind == Builtin` and the name
is in the accepted const-parameter set,” explicitly connecting the two sources of knowledge; the
subset definition for const parameters is validated by the T2 gate (see 5.7).

> **Design judgment**: If the `.yx` corpus contains type annotations such as `i64` / `int`,
> normalizing them to `Int` would change those files' **diagnostic positions** (though not their
> diagnostic codes). C3 permits this (messages and ordering may be normalized), but the baseline
> must be recorded in stage 1. **Measurement (P6 6.0, 2026-10-27)**: A full scan of `tests/yaoxiang`
> (316 `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **0 non-canonical names in actual
> type positions**; 16 raw-token matches were adjudicated individually as 12 strings + 4 comments
> (both `i64` / `f64` occurrences were in comments). Therefore, 6.1's name normalization **does not
> change diagnostic positions in any corpus**, and **no baseline regeneration is required**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned the final **22** spellings—the 2026-10-27
> measurement used a **narrow scope** (counting only 14 integer/floating-point-family spellings and
> excluding namespace usage in `src/std`, hence the record “16 raw-token matches = 12 strings + 4
> comments”). The retest scope was 316 `.yx` files in `tests/yaoxiang` + 39 in
> `tests/yaoxiang-multifile` + 38 in `src/std` = **393 files**, with **308 matching lines**; **only
> 2 type-position candidates existed, and both were in `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`'s `i64::MAX` and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`'s `-> f64`) ⇒ **actual type positions
> still numbered 0**; all other matches were individually adjudicated as string literals / comments
> / the `std.string` module name and namespace access / a `Json` variant name / a `void` literal,
> and **none was a type annotation**. See the 6.1.1 row in [09](09-execution-wbs.md) and 09a's “P6
> 6.1 Name Normalization Closure” for the retest command. As in 6.0, 6.1's name normalization **does
> not change diagnostic positions in any corpus**, and **no baseline regeneration is required**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed loop within the parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injection, no boundary crossing):

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

| File                                                                                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/parser/probe.rs` (new)                                                                     | Define `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is legitimate dependency inversion. **The landed method names are authoritative in this table** (the original design's `is_type_var` / `is_builtin_type` were not adopted: a type variable's identity is determined structurally by its declaration site, not by its name, so this criterion is omitted; built-in-name classification is instead supplied by `kind_of`, and `knows_builtin_types()` declares whether the probe holds the built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                              | Retain `declare_predicate` / `is_predicate_name` (for accumulation within the current pass); add a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**borrow rather than `Box`**). **The landed state also adds a `kind_of()` accessor**—predicates declared in the current pass take priority; all others are delegated to the probe, making this the sole entry point for name classification                                                                                                                                                                                                                                                       |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                 | Delete `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                 | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/parser/ast.rs:930`                                                                         | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                                        |
| All **40** parser construction sites (`ParserState::new` is the sole constructor; **4 production + 36 test**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. The **default implementation, `NullProbe` (all results false)**, ensures parser unit tests remain unaffected; the default probe at production entry points is `BuiltinProbe` (the compiler's built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (The original design's “5 construction sites” was a pre-survey estimate; the measured count is 40—`Parser::new` does not exist, and the only constructor is `ParserState::new`)                                                                                                                                      |

**The `extract_generic_param_names` signature problem**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that accepts no context. After removing the boundary-crossing call at `ast.rs:930`, it
needs an `is_constraint` decision, so its signature must add `probe: &dyn TypeEnvProbe`. Callers,
including `parser/tests/ast.rs`, must be updated to pass `&NullProbe`.

**When `NameKind` is classified**: When `types.rs:162-165` constructs `Type::Name`, it uses
`state.probe()` to determine `kind`, fixing it once so that later match arms no longer perform
string comparisons.

**This eliminates all three entries in RFC-039 routing table C**—two of them already involved only
parser-internal functions under the 2.4 scope; the genuine cross-layer references are the two sites
at `declarations.rs:509` and `ast.rs:930`, both covered here.

### 5.5 Merging `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Claim: delete `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This was already the **running path** (`pratt/nud.rs` constructs
`Expr::Lambda`, and `parser/statements/declarations.rs` constructs `StmtKind::Assign`; see 2.6). The
`FnDef` branch was a never-executed parallel implementation.

**Landed state**: The variant definition and all **15** production consumption arms were **deleted
in the same batch** (Rust's exhaustive `match` makes “definition + arm completion” an atomic unit;
see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5). Baseline consumption sites are
classified by form below:

| Category                                                                       | Count | Disposition                                                                                            |
| ------------------------------------------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------ |
| Variant definition                                                             | 1     | Delete                                                                                                 |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (deletion is required for compilation)                                                          |
| Non-exhaustive match arms + or-patterns with `_ => Span::dummy()`              | 10    | Delete (after deletion they fall through to the fallback; retaining them would leave a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after deletion they never match, with no error or warning)                                     |
| Test construction sites                                                        | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not delete the cases)                                         |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
matching the first three fields of `FnDef`; the `name` field unique to `FnDef` has no available
location at the `Expr` layer (the name resides on `StmtKind::Assign.target`).

**Risks and mitigations (originally planned; now implemented)**: The two non-exhaustive `if let`
sites become impossible matches after the refactoring—without errors or warnings. Mitigation is
provided by **compile-time enforcement**: after the variant is deleted, any future
`if let Expr::FnDef` fails to compile because the variant does not exist. **No gate is needed**.

**Per-site disposition table, test migration, and measured figures**: See
[09a](./09a-wbs-history.md) §P6 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**Delete `StmtKind::Assign` / `StmtKind::TypeDefinition`'s `signature_params: Vec<Param>`.**

Rationale: Its reason for existing (the original comment at `ast.rs:244`) was “for the typechecker's
`classify_generic_params` use”—a **dedicated slot reserved by statement AST for the type checker**,
duplicating the transport of another representation of parameters.

**The implementation scope differs in one critical way from this section's initial draft**
(corrected after stage 1 verification): The initial draft proposed “recomputing `Lambda.params` in
place,” but `Lambda.params` is an **already-filtered** value-parameter list (type-position and
constraint-position parameters removed), while `classify_generic_params` /
`extract_generic_param_names` require the **unfiltered** original signature list. The actual
implementation therefore first moves the names into the sole carrier and then deletes the old field:

1. 6.8.1 first reshapes `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   co-located, reusing the existing `Param`—no second structure and no parallel arrays);
2. 6.5.2 then deletes the two `signature_params`: `Assign` obtains it from `type_annotation`, while
   `TypeDefinition` carries the declaration signature as `type_annotation: Option<Type>` (the same
   carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (expanding each
   curry group in order and treating `Paren`/`NamedParen` as transparent); `NamedParen`'s binder is
   not included and is interpreted explicitly by the type checker (see below).

> **Risk-verification conclusion**: The initial draft's concern about the “unfiltered vs.
> already-filtered” difference was valid—but it was **not a problem**: the flattened result and the
> old `signature_params` are element-for-element equivalent (the same `Param` nodes), so generic
> classification does not change. The evidence is 356/356 corpus files, zero drift in 216 IR
> snapshots, and a fully green `cargo test --all` (record in [09a](./09a-wbs-history.md) §P6 6.5).
> The initial draft's “use `Lambda.params`” approach would have changed generic classification
> results and **was not adopted**.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node and move binder semantics out (completed:
6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records its **sufficient reason for existence** (see 2.7): if `r` is
lost from `(r: P(m))`, the type checker can only guess that “a free variable in a constraint that is
not in scope is the return-value formal parameter,” and would **silently substitute the undeclared
`m` as a binder too** (the comment labels this a “measured defect”).

Therefore, **it cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param`/`param_span`
   carry source facts only and do not carry type-layer binder semantics.
2. Binder interpretation moves to the **type-check layer**: when consuming `NamedParen`, the type
   checker **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node's
   field,” the same kind of relationship as `Expr` embedded in `ConstExpr`—the syntax node provides
   facts, and the semantic layer interprets them.
3. Rewrite the comment from “the type checker can only guess” to “parameter-position formal names
   belong to `Type::Fn.params`; this node carries only the **return-position** binder.”

> **Path correction**: The `sema/` in this section's initial draft is the **target** directory name
> from RFC-039 Decision D1; the repository's current semantic-layer directory is
> `src/frontend/core/typecheck/` (the rename belongs to batch 6.7 and has not yet run). All code
> paths in the body of this document are written against the pre-rename current state (see
> “Directory Renaming and the Path of This Document”).

**Why this is a “migration” rather than a “deletion”**: The syntax tree must represent source
without loss. `r` genuinely exists in the source, so the AST cannot pretend it does not. The problem
is not the node's existence, but the failure to define responsibility for semantic interpretation
clearly.

### 5.7 Build-Time Type-Table Gates

**The reference implementation has already been verified in this repository**: `build.rs:19-37`
invokes `parse_registry` / `validate` from `tools/code-tables` and performs uniqueness, section, and
Chinese-completeness validation over 145 error codes; if `is_ok()` is false, it directly `panic!`s
and refuses to compile; `build.rs:39-54` then compares the Markdown code table in RFC-013 item by
item and likewise `panic!`s on a mismatch. **RFC-013 documentation and code therefore remain
consistent**—`tools/code-tables/Cargo.toml:15-16` has only one `serde_json` dependency, and the
validation logic is text-level parsing with no dependency on syn.

The type tables currently have **no** comparable gate. The mechanism introduced here follows the
same shape.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                              | Check                                                                                                                                                                                                                                                                              | Inputs                                     | Failure condition                                                                                                                                                                                                                                                                                                                                               | Remediation                                                                                                           |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site breakdown**                             | Scan the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant's construction sites in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`) | Variant list + three-category match tables | ① A variant has **zero forward construction sites** and is unregistered; ② a registered variant gains a forward construction site; ③ a registered wildcard-arm fallback exemption no longer matches; ④ an **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered individually and must not be generalized to all wildcard arms) | Delete the variant; or register a rationale / remove an obsolete registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2 Synonym table and repository-wide name-list consistency** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:855` + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                           | Three tables                               | A type name exists in one table but is not recognized by another (compared according to each table's mapping semantics); **since WBS 6.1.2, T2.5 “Deleted Spelling Sentinel” also reports any of the 22 deleted spellings if reintroduced**                                                                                                                     | Reconcile both directions with `type-tables --fix`                                                                    |
| **T3 Built-in type names consistent with RFC-011**             | The canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                               | Two tables                                 | Code-table ranges / names differ from the registry                                                                                                                                                                                                                                                                                                              | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                     |
| **T4 Exhaustiveness of `Type → MonoType`**                     | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                     | Two tables                                 | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                      | Add an arm / delete an arm                                                                                            |

**T4 implementation note**: Rust's exhaustive match already guarantees the “variant → arm” direction
at compile time. T4's value lies in the **reverse** direction—detecting stale arms in `lower.rs`
that point to deleted variants (such code fails to compile when the variant is deleted, so T4 is
effectively redundant). **T4 is therefore downgraded to a CI assertion** (`cargo build` success is
sufficient), and does not enter `build.rs`'s `panic!` path. **T1–T3 are the three items that
genuinely require text parsing.**

**T1's two implementation difficulties and mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has only
   `serde_json`. Mitigations:
   - **Preferred**: Introduce `syn` into `tools/type-tables` only (the root crate remains
     unchanged). `syn` is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: Reuse the repository's existing Python tool `tools/extract_arm.py` and its
     line-level heuristic—a construction site is on the right-hand side of `= ` or in an argument
     position, while a match arm is on the left-hand side of `=>`; use the number of `=>`
     occurrences as an approximation.
   - **Conservative starting point**: When T1 first ships, **report only and do not fail**
     (`cargo:warning`), then establish a full baseline on unmodified code and confirm the heuristic
     before changing it to `panic!`. This matches the handling of “CI gates are initially red and
     need a baseline first.”
2. **Reverse bridges and AST→AST reconstruction sites must be categorized separately**. Section 2.2
   records the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; section 2.2.2 records the two AST→AST replacement functions in
   `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites from the 13-variant list).
   If T1 conflates them with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` /
   `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` will be classified as
   having construction sites and exempted from deletion, directly contradicting the disposition
   table in 5.2. T1's report must separate the three construction-site categories, and **fallback
   exemptions must apply only to individually registered entries** (currently only `Void`'s parser
   error fallback at `types.rs:836`), never generalized to all wildcard arms.

**Gate coverage**: Add a `type_tables::validate(root, &entries)` call in `build.rs`, immediately
after the existing block at `build.rs:19-37`, following the
`panic!("类型表校验失败（{} 项 error ...）", ...)` pattern.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                                                                                 |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `types/substitute.rs:112`'s `substitute`, and `types/mono.rs:649`'s `substitute` all operate on `MonoType`                                                                                                                                                                                                                                                     |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `eval/reducer.rs:572`'s `unify` all operate on `MonoType`                                                                                                                                                                                                                           |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. Eight sites in `mono/function.rs`—`358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`—match `AstType::NamedStruct` / `AstType::AssocType`; `AssocType` was designated for deletion, and all eight sites were updated accordingly. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3; it now uses `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match converged with 6.3; the former direct read of `signature_params` through `let params: Vec<MonoType>` now uses `Type::fn_params_flat` to flatten the parameters (the verification conclusion in 5.6: unfiltered list, equivalent to the old field)                                                                                               |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs modification**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and the assembly at `632`                                                                                                                                                                                                                                                                       |
| Interpreter (`backends/interpreter/`)                        | **Needs modification**. `image.rs:44`'s `type_table: Vec<ir::Type>` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the changed element type affects `{:?}` output                                                                                                                                                                                                |
| RFC-027 compile-time types                                   | **Needs modification**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                                                                                                                 |

**IR-layer type discipline (to be inherited by 04-ssa)**: After convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. The first-layer validator's
“type consistency” invariant in 07 must enforce this discipline. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are the expression-side counterparts of the same problem—they may appear
inside `ConstExpr`, and therefore must likewise never descend into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis:

1. The 13 deleted variants have **zero parser-forward construction sites** (the one forward site for
   `Void` was an error fallback—**WBS 6.3 changed that fallback to `Type::Name{"()"}` and removed
   the gate-exemption registration**, so there are now no forward construction sites) ⇒ the parser
   does not produce them (the reverse-bridge and AST→AST reconstruction sites are covered by #28 /
   #47 in 6.3 and the 10 sites in 6.3.2, all of which are implemented).
2. `ir::Type` is an alias of `ast::Type` ⇒ changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** in `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction in which behavior changes, and it is a fix rather than a
   regression**, but it must be confirmed by the third-layer corpus differential testing in 07.

**Semantic changes requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match is sufficient) |

> **Design judgment**: Because all eight variants currently collapse to `Void`, **every code path
> that depends on signature bytecode** (the interpreter's arity/type checks and the serialization
> format) currently sees `Void`. After convergence, these locations will see the real types. **This
> is the document's only potential source of behavioral change, and its direction is “restore
> correct information.”** If corpus differential testing finds a difference, it should be treated as
> **exposing an existing defect**, not as a regression introduced by this document. Method:
> separately construct a `.yx` program containing a `Ref` parameter and compare `dump_bytecode`
> before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                             | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Category                          |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`           | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**Completed: WBS 6.3.1, 2026-10-29**—all 13 variants deleted, and `ast::Type` converged to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                        | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original variant location, **deleted**) | Delete `Expr::FnDef` (**Completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consumption arms deleted in the same batch, with zero remaining references in `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                        | Delete `Assign.signature_params` (**Completed: WBS 6.5.2, 2026-10-30**—the same-named field was deleted from `TypeDefinition` in the same batch, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm location, **deleted**)      | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3 (completed)            |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                        | Rewrite the `NamedParen` comment to say “syntax provides the binder name; semantic interpretation belongs in the **type-check layer**” (**Completed: WBS 6.5.2, 2026-10-30**—the comment now reads “parameter names belong to `Type::Fn.params`; this node carries only the return-position binder”; the initial draft's `sema/` is Decision D1's target directory name, whereas the repository's current path is `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                                | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                                  | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | New                               |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                              | Add `trait TypeEnvProbe` + `NullProbe` (**Completed: WBS 6.4.1, 2026-10-29**—the landed state also includes `BuiltinProbe` and `NameKind`; the trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | New                               |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                          | Add the `probe` field and accessor (**Completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, and the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites are wired**: 4 production + 36 test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                        | Use `state.probe()` to determine `NameKind` (**Not done**: 6.4 introduced only the **classification enum + probe queries** and **did not change `Type::Name`'s structure**—adding the field would affect every production/test construction site and require the reverse bridge `mono_to_ast_type` to faithfully write back `kind`, forcing IR snapshot and corpus-baseline regeneration, which conflicts with this batch's C3 zero-drift scope; fielding `NameKind` will be reassessed when a second consumer appears)                                                                                                                                                                                                                                                                                                                                                          | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                        | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; add only an explanation of the variant deletion)—**Not done**: 6.4 did not change `types.rs` (that file's change surface belonged to 6.3's fallback refactoring), so this line remains for a later batch together with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                            | Delete the `_ => (Vec::new(), Type::Void)` fallback and use an explicit error return—**WBS 6.3.1 changed the fallback to a let-else explicit return of `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical). **The “change to an explicit error return” half was not done**—it was originally deferred to 6.4's error-path refactoring, but **6.4 did not take over that path** (6.4 only injected the type-name dependency and did not change `types.rs`) ⇒ **6.5 did not take it over either** (that batch only reshaped representations and left the fallback path unchanged) ⇒ **it moved to 6.6**, where it is registered in 09's 6.6.1 row. The path is syntactically reachable (`semantic_tokens`'s `Name` arm records one additional dummy-span token; the corpus has 0 matches, so there is zero drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                        | Delete `n == "Terminates"` and use `state.probe().is_predicate(n)` (**Completed: WBS 6.4.3, 2026-10-29**—the landed implementation reads `NameKind::Predicate` through `ParserState::kind_of`: predicates declared in the current pass take priority, and all others are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                        | Delete the `typecheck::operator_interfaces::spec()` call (**Completed: WBS 6.4.3, 2026-10-29**—now uses `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                            | Delete the `signature_params` argument—**Completed (WBS 6.5.2, 2026-10-30)**: every construction site's field was deleted; the same batch changed the HashMap name table in the name-merging section to positional alignment (criterion: pre-validation guarantees element-wise identical names)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                             | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3 (completed)            |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                            | Retain (the parameter-list retrieval point)—**“Change to read Lambda directly as in 5.5” was not done**: item 6.5.1's change list did not include this site; its disposition is to be decided together with 6.8.2's declaration-driven checking mode                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`           | Delete 13 match arms (one for each of the 13 deleted variants) (**Completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                        | Reduce synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Convergence                       |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                            | Retain the `DateTime` alias                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                              | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | New                               |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                                    | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Convergence                       |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` is a private `use`)        | Delete `pub use ... ast::Type` (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                            | Keep `FunctionCode.params` as `Vec<MonoType>` (already so)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                     | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Convergence                       |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                            | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Convergence                       |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                           | Change the `type_table` conversion to `.collect()` (elements are already the same type)—the actual change is to **delete the conversion and transport the values unchanged** (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)               | **Delete** the entire `From<MonoType> for IrType` impl (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                             | Change `type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Convergence                       |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                            | Handle the `AstType::AssocType` arms according to the gate conclusion—**the conclusion was established and implemented: WBS 6.3.2 deleted all 5 variants / 10 sites registered in 03 §2.2.2** (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each), and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                     | Type-table gates T1–T3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Gates                             |
| 32  | `spawn/placement.rs`                                                   | `122`                                            | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                           | Same, obtaining the name from `Assign.target` instead                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                                   | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                                    | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                        | Delete the `Expr::FnDef` test (**Completed: WBS 6.5.1, 2026-10-30**—the case was not deleted; it was rewritten as two Lambda-destructuring cases plus one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                        | Rewrite as Lambda + Assign (**Completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`                  | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                        | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be handled in the same batch as #1**—**the interpretation has been corrected (Completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is a lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes lowercase spellings it would become a `TypeRef`, **silently losing type identity**. The actual implementation therefore uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                                         | Convergence                       |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                     | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers, including `parser/tests/ast.rs`, pass `&NullProbe` (**Completed: WBS 6.4.3, 2026-10-29**—the landed signature is at `ast.rs:927`; the three production callers in `ir_gen.rs` pass `SemanticProbe`, while the others pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                                  | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                          | Delete this file's `name_used_as_type` and call the merged implementation in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate` (**Completed: WBS 6.4.4, 2026-10-29**—the merged single implementation is at `ast.rs:848` and covers header detection + predicate short-circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                            | Connect the `CONST_PARAM_TYPES` check through `NameKind::Builtin` (see 5.3) (**Completed: WBS 6.4.5, 2026-10-29**—dual-knowledge connection: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe without the built-in-name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Convergence                       |

> **The six landed rows of 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed 2026-10-28)
> implemented these six rows in one batch—four carriers switched to `MonoType` (`bytecode.rs:814`,
> `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType` and the `ir::Type`
> re-export alias were removed, and decoding was changed to transport values unchanged
> (`bytecode.rs:2315`, `:2316`, `:2346`). **The line numbers for these rows have been refreshed to
> the landed state** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390` were
> pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured figures and gate
> results.

> **The gates themselves must also be gated**: `tools/type-tables` requires the same `mod`
> declaration wiring as `code-tables` (the repository has already learned this lesson from eight
> orphaned test trees). Register the new crate in the root `Cargo.toml` at the same time.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/` 293 single-file `.yx` corpus files + the multi-file corpus layer
established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library **must
all retain identical behavior** (identical diagnostic codes and messages, with ordering normalized
as needed).

| Risk                                                                  | Assessment                                                                                                                                                                                                                                                                                                         | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains type annotations such as `i64` / `int` / `f64`   | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                  | **Measured** (end of 5.3): 0 non-canonical names in actual type positions ⇒ 6.1.2 directly deletes the 22 spellings and **does not introduce a lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Source code contains `DateTime`                                       | Retain only the uppercase temporary compatibility alias (`mono.rs:883`); it is not a built-in name                                                                                                                                                                                                                 | Moving it out of the built-in-name table belongs to P10 S11 (issue #513); until then, `DateTime` and `Int` are synonymous, with no behavioral change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `("()")` appears in type position                                     | It is not a product of the `Name` form but a spelling of `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                              | **Retain**: `()` shares an arm with `Void` and is one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `dump_bytecode` output changes                                        | The element type of `type_table` changes from `ir::Type` to `MonoType`, and `{:?}` formatting differs                                                                                                                                                                                                              | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only in C1/C2 stages). However, the output diff must be recorded in stage 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Behavior changes after the lossy bridge is fixed                      | See 6.2                                                                                                                                                                                                                                                                                                            | Corpus differential testing + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Type-replacement results change after `mono_to_ast_type` is rewritten | `function.rs:507-529` is the exit point for monomorphized type replacement (`substitute_type_in_ast:532-537` converts the replaced `MonoType` back to `AstType`). After deleting the variants, using the `_` arm would change the result from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Stage 3 targeted comparison: select corpus files with generic substitution and compare the monomorphized products' `type_name()` before and after the change. **Do not assume `{:?}` output equivalence**. **The interpretation has been corrected (stage 3 measurement)**: Equality must not use `type_name()`—it is a lowercase internal name for diagnostic display, and comparing `type_name()` can only show that both are “`int64`,” not whether the canonical name is `Int64` or `i64`. Instead, compare **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` as mutual inverses for every name) + the structure of the write-back product. WBS 6.3 measurement: the dumps and `.42` product md5 hashes for 4 generic-substitution programs were byte-for-byte identical to the pre-change versions |
| `.yx` source files themselves                                         | **Zero modifications**                                                                                                                                                                                                                                                                                             | This document does not require changes to any corpus or standard-library source file. This is a hard acceptance item                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

**Hard acceptance condition** (**revised 2026-10-27, following the 2026-10-26 Decision ②**):
`git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly driven test
files** under `tests/` may be modified when mechanically required by the change, but each
modification must be registered with its **file / line count / rationale**, and **tests must not be
deleted or assertions weakened**. The original rule (`git diff --stat tests/ src/std/`) applied a
zero-diff requirement to all of `tests/`; this revision **relaxes the rule by one level for directly
driven test files only**. The zero-diff requirement remains unchanged for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`).
**Exception**: the 6.8.2 behavioral-fix batch is handled as a separate round and receives a separate
decision.

## Implementation Essentials

The criterion uniformly used is 07's **C3: identical diagnostic codes and messages, with ordering
normalized as needed**. The following is this workstream's stage breakdown; see RFC-039 for the
global ordering across documents.

| Stage                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Rollback point                                                                                                                                                                                                                                                                                                                                                                                            |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3; the first version of T1 **reports but does not fail**. Establish a full baseline on **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | The T1 report accurately lists the 13 variants with **zero parser-forward construction sites** (including the classification of `AssocType`: 0 forward / 2 AST→AST, D8 unchanged), and **separately reports the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; it reports 13 zero-forward-construction variants separately from the 1 globally zero-construction variant (only `Enum`); T2/T3 are green or provide a repairable list | No code changes; no rollback required                                                                                                                                                                                                                                                                                                                                                                     |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) converged to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; the seven lowercase copies in `add_builtin_types`, the LSP list's `Uint` / `Int128`, and the `sizeof` `Uint` arm were cleaned up in the same batch; **no lexer alias table was introduced** (corpus measurement found 0 non-canonical names). **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**Verified: it uses the unfiltered original signature list; after 6.5.2 it is provided by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                                        | C3: the corpus-wide diagnostic set is **identical item by item** to the stage 0 baseline                                                                                                                                                                                                                                                                                                                                                                                              | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                               |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers switched to `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); removed the lossy bridge `From<MonoType> for IrType` (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias (the `pub use` at `ir.rs:3` was downgraded to a private `use`); decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), and the prior silent information loss in which eight `MonoType` variants collapsed to `Void` no longer occurs                                                                                                                                                                                                                                                                             | C3 + targeted `dump_bytecode` comparison (item by item in the 6.2 table)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                     | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                               |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: delete 13 variants according to the 5.2 table (`ast::Type` 26 → 13); remove match arms such as `mono.rs:699-708` and the fallback at `types.rs:836` (changed to a let-else explicit return of `Type::Name{"()"}`); **handle `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; update `types/tests/mono.rs` accordingly. **#47's interpretation was corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” became invalid after 6.1—`type_name()` is a lowercase internal name for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes lowercase spellings it would become a `TypeRef`, **silently losing type identity**; the actual implementation uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back | C3 + a fully green `cargo build` (exhaustive matches force completion of every arm) + targeted comparison of generic substitution (last row of 6.4)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                          | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                               |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: add `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); change `parser_state.rs` (add the `probe` field and `probe()` / `kind_of()` accessors; **all 40 construction sites wired** = 4 production + 36 test); delete the parser's two `operator_interfaces::spec()` calls and change one middle-layer site and one driver-layer site to the narrow `is_constraint_name` entry point (#48); merge the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connect `CONST_PARAM_TYPES` through `NameKind::Builtin` (#50). **Not done**: adding `kind` to `Type::Name` at `types.rs:162-165` (#10; fielding conflicts with this batch's C3 scope), and changing the fallback at `types.rs:836` to an explicit error return (#12, moved to 6.5)                                                          | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps (`typecheck` / `operator_interfaces`) measured 0 matches**; 356/356 corpus files, zero drift in 216 IR snapshots, and `cargo test --all` exited 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                        | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                               |
| **5. Dead AST variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: delete `Expr::FnDef` (baseline measurement: **25 sites / 16 files** = 1 definition + 15 production consumption sites + 3 test construction sites + 6 comments, of which 19 were code references) + delete the parallel parameter tables in `Assign` / `TypeDefinition`; reshape `Type::Fn.params` into `Vec<Param>` (names and types co-located) ⇒ the sole flattening entry point `Type::fn_params_flat`; retain `NamedParen` as a syntax node and have the type checker explicitly interpret its binder                                                                                                                                                                                                                                                                                                                                   | C3 + all **22 modules** under `tests/integration/` green (measured: 22 `.rs` files; the original text's 18 was an old count); formatter parameter-position output byte-for-byte identical (all 355 `.yx` corpus files compared byte-for-byte); 356/356 corpus files, zero drift in 216 IR snapshots; `#[test]` count in `src/` 2878 → 2883 (no tests deleted)                                                                                                                         | Seven commits (2 variant-deletion commits + 3 representation-reshaping/old-representation-deletion commits + 2 fix commits); **revert scope**: the variant-deletion commit is independently revertible, and the test-migration commit must be reverted in tandem with it (see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5); measured figures and gates in [09a](./09a-wbs-history.md) §P6 6.5 |
| **6. Convert gates to hard failures**         | Change T1 from warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                              | Separate commit                                                                                                                                                                                                                                                                                                                                                                                           |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage 2**—before stage
2 lands, reverse bridge A on the bytecode side (`bytecode.rs`'s `From<MonoType> for IrType`)
consumes `IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation;
that bridge was removed in 6.2.2. **The remaining rewrite surface when stage 3 deletes the variants
is therefore reverse bridge B** (`mono_to_ast_type` in `middle/passes/mono/function.rs`, which still
resolves `String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`.
Stage 3's #47 must be handled in the same batch as #1; otherwise, `mono_to_ast_type` silently
changes behavior. **Stage 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B now uses
`canonical_name` + structural write-back; the three `ast::Type` carriers in `middle/core/ir.rs`
**were not changed**—6.3 only deleted variants and did not alter the IR shape (zero drift in 216 IR
snapshots and 356 corpus baseline lines), and their disposition was deferred to stage 4.

**Dependency on downstream work**: Only after all stages in this document are complete may
[04-ssa.md](04-ssa.md) begin. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all have the same element type, giving SSA type annotations a single destination.

### Mandatory Validation for Each Stage

| Stage | Required action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction breakdown**)                                                                                                                                                                                                                                                                                                                                      |
| 1–5   | `cargo test`; full corpus differential testing over `tests/yaoxiang/` (third layer of 07); compare the C3 criterion item by item                                                                                                                                                                                                                                                                                                                                                                     |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                                   |
| 3     | Additionally: for corpus files containing generic substitution, compare the monomorphized products before and after the change by `type_name()`—**the interpretation has been corrected (stage 3 measurement)**: `type_name()` is a lowercase internal name for diagnostic display and is insufficient for equality; instead compare **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` as mutual inverses for every name) + the structure of the write-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                                      |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                                  | Rationale                                                                                                                                                                                                                                    |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                                                                                                                                                   | The alias at `ir.rs:3` directly causes the field-type mismatch between `BytecodeFunction.params` and `FunctionCode.params` (2.1). Once the alias is deleted, the two field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                                 | It would create an L2→L3 reverse dependency, violating RFC-039's layering, and would require adding `Span` / `ConstExpr` back to `MonoType`. See the complete argument in 5.1                                                                |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and move binder semantics to the **type-check layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is Decision D1's target directory name, whereas the repository's current directory is `typecheck/`) | `ast.rs:518-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as a binder                                                                                |
| Directory renaming           | **Do it** (D1), executed as a pure move batch at P5/P6 closure; paths in this document's body use the pre-rename current state                                                                                                                              | 2026-10-03 decision (ChenXu233); construction method clarified 2026-10-04                                                                                                                                                                    |

### Rejected Alternatives

None of the four alternative approaches was adopted: **retaining three representations and adding
only a conversion layer** resolves none of the root causes, and `ir::Type` would still not be an
independent type; dead variants, the synonym table, and the reverse dependency would all remain;
**using a macro to generate the synonym table** treats a symptom rather than the cause—after
`Type::Int(usize)` is deleted, the macro has no reason to exist, and consistency should instead be
guaranteed by gate T2 (more verifiable than macro expansion correctness); **deleting only dead
variants without unification** has the lowest cost but leaves the problem in place, with no gate
preventing the next person from adding another `Type::Int128(usize)` and recreating the same dead
variant (if only one commit can be made, it should explicitly be recorded as a subset of this
approach and include the T1 gate as a deliverable); **unifying on `MonoType`** is discussed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. `Expr::FnDef`'s 15 production consumption
  sites (including 3 exhaustive arms) + 3 test construction sites + 1 variant definition form a
  never-executed parallel path; deleting it removes a branch from each of `ir_gen.rs` /
  `ownership.rs` / `termination.rs`.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  already differ in definition-body field recursion; after merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value.”** This is
  the prerequisite for 07's first-layer “type consistency” invariant to pass genuinely.
- **The gates make regression impossible**. T1 changes the addition of a new zero-construction
  variant from silently passing to failing the build—the repository's existing `build.rs` mechanism
  demonstrates that this approach works.
- **Zero source-code changes**. The 293 `.yx` corpus files and the standard library require no
  modifications, which is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may change behavior**. After the lossy bridge is fixed, eight `MonoType` variants expose
  their real types rather than `Void`. The direction is a fix, but it changes paths that depend on
  signature bytecode. This is the document's largest uncertainty.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (such as counting `=>`) rather than `syn`, it may misclassify. This is why stage 0 requires manual
  verification.
- **If T1 omits reverse bridges, the disposition table and gate contradict each other**. Section 2.2
  proves that seven “dead variants” have production reconstruction sites in reverse bridges. If T1
  does not separate construction-site directions, those seven variants are classified as having
  construction sites and exempted from deletion—the gate then obstructs the deletion work instead of
  safeguarding it. | **2. Deleting `signature_params` carries semantic risk** (**verified and
  resolved: 6.5.2, 2026-10-30**). It carries the **unfiltered** parameter list, whereas
  `Lambda.params` is **already filtered**. If `classify_generic_params` depends on the former, the
  refactoring changes generic classification results. **Measured conclusion**:
  `classify_generic_params` / `extract_generic_param_names` need the unfiltered list; the flattened
  `Type::Fn.params` and the old `signature_params` have the same source (the same `Param` nodes), so
  classification does not change. The “use `Lambda.params`” approach would change behavior and **was
  not adopted** (356/356 corpus files, 216 snapshots, fully green `cargo test --all`). | **3. Stage
  3 touches `middle/passes/mono/function.rs`**. Besides the eight `AssocType` match sites,
  `507-529`'s `mono_to_ast_type` is the exit point for monomorphized type replacement, and rewriting
  it may change the replacement result's `type_name()` (last row of 6.4).
- **Non-canonical type names in the `.yx` corpus may be nonzero**. If the corpus extensively uses
  `i64` / `int`, directly deleting synonyms in stage 1 is infeasible and must fall back to “move
  them into a lexer alias table,” reducing the scope of `from_builtin_name`'s convergence.
  (**Measured as 0**: the 6.0 survey at the end of 5.3 established this, so WBS 6.1.2 directly
  deleted the aliases and did not take the alias-table route.)
- **The gates are initially red**. If T1 ships with an immediate `panic!`, it will block
  compilation. It must first report only and establish a baseline.

### Open Questions

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This document contains no remaining pending items.**

## See Also

### Higher-Level and Same-Batch Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — Higher-level master plan; four-layer model, criterion tiers, and routing table C (three
  type-representation-related entries are assigned to this document)
- [01-routing.md](01-routing.md) — Feature routing tables A/B/C, dependency-direction rules, and
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Front-end paradigm; convergence of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definition of C3, diagnostic-set comparison
  rules, and the first-layer IR validator's type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Built-in type table
  (comparison target for gate T3)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of a build-time gate in `build.rs`
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line anchors refreshed to the 6.5 landed state** (2026-10-30); entries deleted in earlier
> batches are retained and annotated with their deletion batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after 6.3
  closure; the 26-variant era used the range 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — Evidence that deleting `Type::NamedParen`'s binder
  semantics would reintroduce a defect
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types co-located) and
  `Type::fn_params_flat` (flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- `src/frontend/core/parser/ast.rs:858` — `Type::without_fn_param_names`: strips type-position
  parameter names before IR carriers enter the IR (printing-boundary consistency, see 5.6)
- `src/frontend/core/parser/ast.rs:1074` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single merged implementation
  after 6.4 (the former `name_used_as_type_in` and the copy in `declarations.rs` were merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through an injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — The parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: fallback for non-`Fn`
  forms (since 6.3, returns `Type::Name{"()"}`; the `Type::Void` variant was deleted)
- (**Deleted, 6.4**) `src/frontend/core/parser/statements/declarations.rs:42-80` — Original
  `name_used_as_type`, merged into the single implementation in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — Former hard-coded `"Terminates"` and
  `typecheck::operator_interfaces::spec()` call (6.4 switched to `ParserState::kind_of` / the
  injected probe; both literal references disappeared, leaving only a comment here)
- `src/frontend/core/parser/pratt/nud.rs:512` — Expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — `expr_to_params` arm retrieves the parameter list
- `src/frontend/core/types/mono.rs:863` — `from_builtin_name` built-in-name resolution table (15
  names + temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:960` — `From<ast::Type> for MonoType` implementation (12 of the
  13 match arms are here)
- `src/frontend/core/types/mono.rs:169` — `MonoType` definition (approximately 22 variants)
- `src/frontend/core/types/const_data.rs:240`, `:327` — `const_data::BinOp` / `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- (**Deleted, 6.2.2**) `src/middle/core/ir.rs:3` — Evidence that `ir::Type` was an alias of
  `ast::Type`
- `src/middle/core/bytecode.rs:809` (`BytecodeFunction`), `:814` / `:816` (`params` / `return_type`,
  carrying `MonoType` since 6.2)
- (**Deleted, 6.2.2**) `src/middle/core/bytecode.rs:2352-2390` — Lossy `From<MonoType> for IrType`
  bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `:275-278` — `BytecodeFile::type_table` and the
  `Vec<MonoType>` fields of `FunctionCode`
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: reverse bridge rewritten in 6.3
  (`canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — Interpreter module's `type_table`
- `build.rs:33-51` — Build-time gate for the error-code registry (shape reference for the type-table
  gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — The gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
