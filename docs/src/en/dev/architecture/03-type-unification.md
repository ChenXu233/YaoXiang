# Unification of Type Representations

> **Ancillary design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, tiered acceptance criteria, and execution-phase
> order; see the [index for this directory](index.md) for the positioning of each ancillary
> document.

## Scope and Positioning

### Coverage

Consolidate the multiple parallel type representations currently in use into one, thereby
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero parser-forward construction sites** (among the 26 variants in
  `ast.rs:427-541`; see 2.2; the “forward” metric excludes reverse bridges and AST→AST
  reconstruction points)
- **A hand-written synonym table for type names** (`mono.rs:839-886`; see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser's genuine reverse dependency on the typecheck layer** (2 instances; see 2.4)
- **One expression variant with zero production construction sites**, `Expr::FnDef` (**removed: WBS
  6.5.1, 2026-10-30**—the variant definition and every consuming arm were deleted; see 2.6)

The concrete deliverables are four items: a disposition table covering each of the 26 variants
(5.2), a semantic definition of `Type::Name` through `NameKind` and elimination of the synonym table
(5.3), an injected data flow through `TypeEnvProbe` (5.4), and build-time type-table gates T1/T2
(5.7; the original T3 was deleted following the user ruling of 2026-10-30).

### Directory Renaming and This Document's Paths

RFC-039 decision D1 requires renaming directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`), **to be completed together with P5/P6, without phasing**. This
document's 50-item change list and all acceptance greps **use the pre-rename paths as they exist
today**—they constitute the construction baseline for P6 batches **before the renaming batch**; the
directory rename at the end of each batch is a **pure relocation batch** (C1 zero-diff) in a
separate commit. The baseline becoming invalid after the rename commit is expected; use git history
as the bisection boundary.

### Out of Scope

- **How the directory rename is performed**. `Type` is not moved out of `parser/ast.rs` during this
  phase; only `lower.rs` is added under `types/` as the sole conversion point. The directory rename
  (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 under RFC-039 decision D1; its
  target structure is shown in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Tiering equivalence criteria and validator implementation**. The definitions of C1-C6 and the
  three tiers of criteria (IR validator / normalized snapshots / corpus differential) are defined in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document cites only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is a prerequisite for it, for the
  reasons described below.
- **The global P1-P10 execution sequence**. See the main body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document gives only the phase breakdown for its
  own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the overarching framework: it defines the four-layer model, criterion tiers,
cross-document phase ordering, and “why perform this refactoring now.” This document is the
construction plan for **one part of RFC-039—type-representation convergence**: exactly which
variants to delete, which files to change, in what order, and which gates prevent regressions.

Every line number, variant list, and construction-site count in this document comes from static
verification of `src/`; each item provides `file:line` evidence. **This document does not record the
execution process; it records code facts and the dispositions derived from them.**

For convenient cross-referencing with documents from the same batch, the “Target Design” and
“Detailed Design” chapters retain the `5.x` / `6.x` subsection numbering used elsewhere, while the
remaining chapters follow this directory's standard format.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(the `pub use` line at the head of `src/middle/core/ir.rs` before 6.2.2; that alias has since been
removed, so it is no longer referenced by line number below). [04-ssa.md](04-ssa.md) is to introduce
register type annotations and new instructions into the IR. If the representations are not
consolidated first, SSA type annotations will follow the `ir::Type` alias directly onto the
syntactic type with 26 variants, effectively institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree inside a type.
  An `Expr` appearing in an IR instruction's type annotation is impossible.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently entrench the 13 dead
variants and the two syntax-specific variants in the IR contract.

## Current State

Everything below is a statement of code fact, not a proposal. Each item provides `file:line`
evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 landed)**: this section records the state **before** 6.2
> landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed four type carriers to carry `MonoType`, deleted
> the lossy bridge `From<MonoType> for IrType`, and removed the `ir::Type` re-export alias. Every
> statement below concerning `ir::Type` or the lossy bridge is therefore a **pre-change fact**; the
> disposition marked “(deleted in 6.2.2)” is already in effect. See [09a](./09a-wbs-history.md) §P6
> 6.2 for implementation details and measured results.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                               | Causal relationship to type representations                                                                                                                                                                                                                                                                                                             |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, and the parser selects the latter, typecheck must supply a table to translate the former back                                                                           |
| Bytecode-layer signatures differ from IR-layer signatures        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Thus `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a builtin type name requires multiple uncoordinated edits | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and a string match in `bytecode.rs:2371-2376`; each location duplicates the mapping, with no cross-validation                                                                                                                         |
| The parser hardcodes `"Terminates"`                              | Because the parser cannot determine “is this a type?” or “is this an operator interface?”, it embeds type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                           |

**The first three are representation problems; the fourth is a downstream consequence of a
representation problem.** Doing only item 1 (deleting dead variants) leaves the synonym table and
reverse dependency in place; doing only item 4 (injecting a callback into the parser) builds on an
inconsistent representation.

One of the three parallel representations is an alias illusion:

| Name        | Definition site                                                            | Size        | Role                                                    |
| ----------- | -------------------------------------------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541`                                  | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`                                          | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | Before 6.2.2: head of `src/middle/core/ir.rs` (that line has been removed) | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

Before 6.2.2, the head of `ir.rs` began with a `pub use … parser::ast::Type;` line followed by a
`use … typecheck::MonoType;` line. The IR module depended on two “type” symbols at once, even though
one was merely an alias. (Both lines disappeared with removal of the alias, so they are no longer
referenced by line number.)

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was deleted in 6.2.2).** `From<MonoType> for IrType`
(previously at `src/middle/core/bytecode.rs:2353-2390`) degraded `MonoType` into `ast::Type`, with
**silent information loss**:

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

All 8 `MonoType` variants collapse into `IrType::Void`, in addition to the `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; its comment acknowledges that this is a “transitional branch
before removing enum variants”).

`IrType::Void` is here a **legitimate type value, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function's return type was lost
during bridging”—one reason why the first-layer “type consistency” invariant in 07 cannot currently
pass on this code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants addressed in 2.2—**both
ends of the lossy bridge point to dead code**.

**Current state (from 6.2.2 onward)**: the impl has been **removed in its entirety**. Decoding no
longer degrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now transports the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those 8
`MonoType` variants retain their real types in signatures and type tables, and `IrType::Void` no
longer doubles as a substitute for “information loss.” The `ir::Type` re-export alias (the
`pub use ... ast::Type` line at the head of `src/middle/core/ir.rs` before 6.2.2) has also been
demoted to a private import—the IR layer no longer publicly re-exports a syntax-layer type. The IR
layer previously had three carriers that directly held `ast::Type` (`Instruction::Cast.target_type`
/ `Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`): the first two were deleted in
**WBS 6.6.11** together with the two **zero-producer** dead variants (their payloads were already
discarded downstream); the last was replaced in **WBS 6.6.12** with the IR-native narrowed carrier
[`TypeDeclBody`] (field table `Vec<TypeDeclField>` + bytecode type-table entry
`self_type: MonoType`)—**the IR layer no longer holds any `ast::Type`, and `src/middle/core/ir.rs`
no longer imports the syntax-layer `ast`**. The **bytecode-side `BytecodeInstr::Cast` of the same
name is a different construct** (its payload is a type-table index `u16`, not a syntactic type) and
remains unchanged.

### 2.2 Parser-Forward Zero-Construction Variants

**Definition of the metric**: the construction-site scan covers all of `src/`, excluding `tests/`
and `*/tests/*`. **Construction site = a `Type::<variant>` path in expression position** (pattern
positions do not count), and sites are listed in **three separate categories that must not be
conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero parser-forward construction sites**
(reconstruction sites in the other two categories are described in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction site                                                                                                                                                                                                                                                                             |
| ------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                          |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                          |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                          |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                          |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the sole forward site = error fallback** `_ => (Vec::new(), Type::Void)`, individually registered as an exemption; eliminated with 6.4 #12)                                                                                                                                                |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                          |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites are described in 2.2.2                                                                                                                                                                                                                   |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites are described in 2.2.2                                                                                                                                                                                                                   |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites are described in 2.2.2                                                                                                                                                                                                                   |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites are described in 2.2.2                                                                                                                                                                                                                   |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 AST→AST production reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Definition-column line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the
> “Definition” columns in this table and in the 5.2 table were written on 2026-10-05. Seventeen
> lines were subsequently inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to
> 444), so **the definition line of all 26 variants shifts uniformly by +17** (for example, `Int`
> moves from 432 → 449 and `NamedParen` from 533-540 → 550-557). This round only **registers** the
> drift and does not alter the table body; subsequent references use the `ast::Type` symbol as the
> anchor.

The causal chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   states explicitly: “RFC-010: Result/Option are not lowered into dedicated AST nodes… they follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The sole forward construction site of `Type::Void` is an error fallback**—`types.rs:836`'s
   `_ => (Vec::new(), Type::Void)`, located at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants do have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
The formulation in this document is therefore “**zero parser-forward construction**”: it means
neither “zero references globally” nor “zero production construction”—11 of these items do have
production construction sites in the other two categories (reverse bridge / AST→AST reconstruction).
These tests themselves codify the behavior under test: they assert exactly what these syntactic
forms lower to as `MonoType`, even though the parser never produces those forms. When deleting the
variants, these tests must be deleted or rewritten as well.

#### 2.2.1 Metric Boundary (1): The Reverse Bridge Reconstructs 7 of Them

The “zero forward construction” claim in the table above holds only in the **parser → AST
forward-construction** direction. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct 7 of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant set                                                                              |
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

**This metric boundary directly affects the disposition**:

- The `bytecode.rs` site disappears with 6.3 #28 (deletion of the entire `From<MonoType> for IrType`
  impl), so it adds no separate cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name resolver (annotated `#299`) that resolves `String` / `Bytes` from `Generic` back into
  dedicated variants. After deleting `ast::Type::String` / `Bytes`, these two guard branches must be
  rewritten; after deleting `Int` / `Float` / `Bool` / `Char` / `Void`, the behavior of the `_` arm
  changes (some types that previously landed in `_` will silently take the `AstType::Name` path).
  This is item 47 added to 6.3.
- **Direct consequence for the gate**: a T1 that scans only forward construction sites would
  classify the above 7 variants as “having construction sites” and thereby exclude them from the
  deletion list. T1 must explicitly include reverse bridges in its counting metric; otherwise its
  conclusions contradict the disposition table in this document (see 5.7).

#### 2.2.2 Metric Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution (recreating an
existing AST type; they are **not** `MonoType → AST`). P6 T1 gate measurements (6.0.2 baseline,
2026-10-27): **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to the 13-variant
list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these 5 variants, these 10 sites must be rewritten in the same batch** (09 §6.3.2
already registers them); otherwise the corresponding arms will no longer compile. **Implemented (WBS
6.3.2, 2026-10-29)**: all 10 sites were deleted; see the disposition table in
[09a](./09a-wbs-history.md) §P6 6.3.

### 2.3 Hand-Written Synonym Table

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

**The sole reason this table exists is to bridge an AST-layer inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent at the AST layer but
structurally different (and the parser produces only the latter), `from_builtin_name` must accept
capitalization variants, abbreviations, signed names, and the `DateTime` alias. The presence of
`"()"` on the `Void` line is particularly telling—it cooperates with the `Type::Void` fallback at
`types.rs:836`.

The same knowledge—“which names are builtin type names”—has seven copies in the repository, with no
consistency validation among them:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05 and its original line numbers have drifted)             | Form                                                                                                                                                                                                                        |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                             |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, copying `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                                    |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                    |
| 4   | `src/lsp/world.rs:182`                                                                                                            | LSP-side list of builtin type names (14 entries = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                       |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` alias families (registered by the P0 gate on 2026-10-06)                                              |
| 6   | `eval_builtin`'s base-type size table (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                            | String match, name → byte size. It previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 deleted that arm, and `sizeof(Uint)` now reports Unknown type. **Dead arm; living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, builtin name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set except by name” decision (`inference/expressions.rs:2917-2918`, decision 4 of #300)                                      |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223; the others by 2-9 lines each), but **the
> item counts and content have not changed** (item 1 had 13 arms when this section was written; item
> 3 has 15 entries; item 4 had 15 entries when written; after WBS 6.1.2, item 1 has 15 arms / 16
> spellings and item 4 has 14 entries). Subsequent references use **symbol anchor + repository
> path**; the line numbers in this table are for location only. The arm-count drift in item 1 and
> the live drift evidence in items 5 / 6 / 7 (`Uint` dead arm, `Set` contradiction with typecheck)
> **all remain**, and this section does not change code.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to identify const generic parameters. It
enumerates “type names usable as const parameters,” which is **partially overlapping with but
different from** the full set handled by `from_builtin_name`, with no mechanism ensuring
synchronization. Item 4 is a fourth copy used for LSP syntax highlighting and hovers, likewise
without validation (T2's comparison scope includes it).

Items 5-7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 / 7 share the same root as items 1-4
(builtin-name → semantic/ID string-key tables), although their purposes differ and they share the
same drift risk. The same gate also found 11 string dispatches for builtin container names on the
typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4, etc.)—these
are **outside both this section and the parser data-flow scope in 5.4**. Their inventory and
dispositions are registered in [06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 under the P6
extension verification items.

### 2.4 The Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation`
/ `name_used_as_type`, both of which belong to the type layer). **Measurement differs from that
description**: these two functions are defined inside the parser itself, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the
  repository)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the only `fn` definition in the
  repository)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are only **2 production-code cross-layer references** to `typecheck`, and both call the same
function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                 | Purpose                                                                                              |
| --- | --------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Identifies constrained-position parameters (`T: Add`), which do not occupy runtime parameters        |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Identifies constrained-position parameters and produces `GenericParamName` values with `constraints` |

The second site is deeper: **`parser/ast.rs` itself** (the entire `ast.rs:847-948` block alongside
`StmtKind` / `Expr`) directly references a typecheck symbol. Routing table C records only the
`declarations.rs` site.

Together with the hardcoded logic at `declarations.rs:498-499`, this forms one judgment:

```rust
// declarations.rs:498-499
// 随后续内置谓词增加而扩展（未来的开集方案：把谓词名下传给
// parser，或改成语法层可判定的形态）。
// parser 不持有类型环境，故看两处：内置谓词硬编码，
// 外加本遍已解析的声明累积（`declare_predicate`）。
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment itself acknowledges this as a temporary form for a “future open-set solution.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask “is this an
operator interface name?” because it must distinguish whether `Type::Name` denotes a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least 4 meanings** (concrete type / type variable / constraint name / predicate name). The syntax
layer cannot determine this by itself, so the authority is pushed upward into the type layer.

#### Duplicate Implementation: Same Name and Meaning, Already Diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` acknowledges this:

```rust
/// 参数名是否在给定类型中被当作类型引用（`(N: Int) -> (n: N)` 的 `N`）。
///
/// 与 `declarations.rs` 的 `name_used_as_type` 同义，此处独立实现以避免
/// parser 内部跨模块依赖。
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` in `declarations.rs:42-80`                            | `name_used_as_type_in` in `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                   | No callback                                          |
| `Type::Generic` branch  | Short-circuits on `is_predicate(app_name)` and returns `false` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls through to `_ => false`)                                | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records the origin of the latter two branches: “Previously these two
branches were missing, so when const generic parameters were referenced in a definition body, it
could not be determined that they were ‘used as types’; annotation validation would falsely report a
legal const parameter as an unknown name.”

In other words: **the function was duplicated to avoid a cross-module parser dependency, but its
fixes were never backported to the original.** The `ast.rs` version recursively handles fields in
definition bodies, while the `declarations.rs` version does not—the same source fragment yields
different results at two judgment sites. This duplication must be consolidated when `NameKind` is
introduced in 5.3.

### 2.5 Parallel Operator Enums

There are two operator enums, which do not reference each other and **have already diverged in their
members**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                  |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Each also implements its own complete `Display` (`const_data.rs:291-317`) and classification
predicates (`const_data.rs:260-289`'s `is_arithmetic` / `is_comparison` / `is_logical` /
`is_bitwise`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they appear in type position, they fall
into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This directly relates to the
IR-layer type discipline in 6.1: after consolidation, `Type::ConstExpr` disappears once
`Type → MonoType`, and the IR can never see it, while `Assign` / `Deref` continue to exist through
paths such as `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
Because the two enums have diverged, neither side alone can determine which operators may occur in
type position.

This phase does not merge the two enums (their ownership is covered under “operator change surface”
in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but T1's scan targets should include
`ast::BinOp` / `ast::UnOp` so that newly introduced zero-construction operators become equally
visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction (Removed: WBS 6.5.1, 2026-10-30)

**This section records an already-landed fact**: `Expr::FnDef` used to be a variant of `ast::Expr`.
It is true that it had **zero production construction**—the only two construction sites in the
entire repository (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`) are tests.

However, **there were far more than 4 consumption sites**: the baseline measured **15 production
consumption sites** (classified in 5.5), plus 6 comments mentioning the variant. At the 6.4 closeout
state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1 definition + 15 production
consumptions + 3 test constructions + 6 comments.

**The path actually taken by function definitions** is: `pratt/nud.rs` constructs `Expr::Lambda`,
and `parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in
`value`). In other words, the `FnDef` branch was **a never-executed parallel path maintained by 19
code locations** (15 consumptions + 3 test constructions + 1 definition).

**Landed state**: the variant definition and every consuming arm were **deleted completely**;
`git grep -n "Expr::FnDef" -- src/` within `src/` now returns 0 matches (only 2 explanatory comments
containing the bare word `FnDef` remain). See [09a](./09a-wbs-history.md) §P6 6.5 for the per-site
disposition table, test migration, and measured results; see 5.5 for the design of the
consolidation.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` once carried a typechecker-specific field (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>`
(originally `ast.rs:244-245`), and its comment stated outright that “the first parameter group of
the signature is preserved as-is, including parameter names, for the typechecker's
`classify_generic_params`”—the statement AST reserved a dedicated slot for the type checker.

Current representation: parameter names and types **co-reside** in the type annotation's
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` now carries the declared
signature as `type_annotation: Option<Type>` (the same carrier used by `Assign`).

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and treats it as a parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module's match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:518-532` states explicitly:

> RFC-027 §3's return-position refinement relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable not in scope in a constraint is the return-value formal parameter”; consequently,
> `(r: P(m))` would silently substitute the undeclared `m` as a binder as well (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Position: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is demoted to the type checker's internal working representation; the `ir::Type` alias is
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

**Why not “merge into `MonoType`”** (this is the easiest proposal to raise and is rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), while
   `MonoType` does not. C3 in 07 requires “identical diagnostic codes and messages”—diagnostic
   positions must remain traceable to source. Adding `Span` to `MonoType` would attach a collection
   of meaningless sentinel spans to every monomorphized type.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker's solving
   process**, not types themselves. Putting them into the sole type structure would require the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) and are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counterargument**: elevating `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer's type. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this document's objectives, so
   choosing a design that creates one would be self-contradictory.

**Why `ir::Type` must be deleted outright rather than “retained”**:

The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) made
`BytecodeFunction.params` (`bytecode.rs:812`) and `return_type` (`bytecode.rs:814`) use `ir::Type`,
while the serialization side (`codegen/bytecode.rs:277-278`) used `MonoType`. The two field types
were connected by `bytecode.rs:2339`'s `file.type_table.into_iter().map(|t| t.into())` and the lossy
`From` bridge at `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be deleted in its entirety
(along with the reverse reconstruction recorded in 2.2).

**Explicitly out of scope**: do not move `Type` from `parser/ast.rs` to `sema/types/`. This phase
only adds `lower.rs` under `types/` as the sole conversion point, leaving `Type`'s definition site
unchanged; the directory rename is performed at the end of P6 under D1 as an independent relocation
batch—afterward, `Type` belongs at the AST top level (`ast/type_.rs`), while `sema/types/` is home
to the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> **Landed (WBS 6.3, 2026-10-29)**: all 13 “Delete” items in this table **have been
> implemented**—`ast::Type` was consolidated from 26 variants to 13 (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were **rewritten in the same batch**. See [09a](./09a-wbs-history.md) §P6 6.3 for
> measured results, the 10-site disposition table, and the criteria.

> Disposition categories: **Delete** (zero parser-forward construction), **Retain** (has forward
> construction sites), **Migrate** (semantics belong to another node).

> **Line-number convention**: the “Definition line” column gives each variant's **original position
> before deletion** (the 26-variant era; 6.3 deleted 13 of them); the matching-site line numbers in
> the “Rationale” column use the same **pre-6.3 baseline** and do not shift with code movement. This
> batch only refreshes the definition lines for the variants that still exist (rows 14 / 26) to
> their landed positions and corrects the now-invalid consumption site in row 14.

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthen)                    | The parser's **sole** primitive-type exit (`types.rs:162-165`). After deleting the 12 primitive-type variants, all primitive types take this path. Add a “semantic kind of name” annotation (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:699` is a match arm). Bit width is carried through the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it, so that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:703-706` is a match arm). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The sole parser-forward construction site is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, individually registered as an exemption). That fallback is eliminated together with 6.4's error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction site `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:743-746` is a match arm); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) are rewritten with 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:747` is a match arm); all three categories total 0 (the sole globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; this is unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                                 | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering) (the original `bytecode.rs:2366` referred to `IrType::Fn` and disappeared with 6.2's `ir::Type` alias and lossy bridge); **the field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types co-reside) lands with 6.8.1 (D54)**—`Assign.signature_params` names are carried over accordingly (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous`/`from_type`/`is_named`; unnamed parameters (pure type positions) are synthesized with empty names, leaving this table's “net effect” unchanged; `Param` has a handwritten `Debug` implementation (unnamed parameters print using their carried type ⇒ zero drift in IR snapshots, see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) are rewritten with 6.3.2. `types.rs:392-396` already explains that the `Generic` path is used                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) are rewritten with 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero parser-forward construction (the syntax layer has no path producing `::` associated types); **2 AST→AST production reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten with 6.3 (registered in 6.3.2). **D8 unchanged**: the metric is forward construction; if associated-type syntax is enabled in the future, a new proposal will define it                                                                                                                                                                                                                                                                                                                                                                                                       |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) are rewritten with 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction site `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type in unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has production construction site `types.rs:160`. It is the only `Type` variant embedding an `Expr`, and the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Semantics (binder name) move to the type checker; the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now says “parameter names belong to `Type::Fn.params`; this node carries only the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction must be handled before variants are deleted**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs 7 of rows 2-8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 has 5 variants / 10 sites within
> the 13-variant list (Union/Option/Result/Sum/AssocType, 2 sites each). Both categories must be
> handled in the same batch as 6.3.1; otherwise the result is either a silent behavior change (see
> 6.3 #47) or a direct compilation failure.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly under D8 (row
> 18 of the 5.2 table). The **metric is parser-forward construction**: zero forward construction
> holds (the syntax layer has no path producing `::` associated types), so D8 need not be changed
> and the variant does not enter the allowlist. The **2 AST→AST production reconstruction sites**
> (`function.rs:456`'s `substitute_type_ast` and `function.rs:623`'s `substitute_type_in_ast`) are
> registered in 6.3.2 and are rewritten with 6.3. T1's three-category split (5.7) reproduces this
> conclusion (commit measurement 6.0.1: 26/26 variants match one-for-one).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least 4
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine “is this a predicate?”,
`510-512` in the same file must determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` must determine “is this an operator interface?”, and
`name_used_as_type_in` (measured at 864) in the same file and `name_used_as_type` at
`declarations.rs:42` must determine “is this a type reference?”—all through string comparisons.

**Proposal: add a `kind` field to `Type::Name`, assigned once by the parser.**

```rust
// 目标形态（示意；行号为改动后）
Name {
    name: String,
    kind: NameKind,   // 新增
    span: Span,
}

enum NameKind {
    Builtin,      // 内置类型名：Int / Float / Bool / ... → 交给 from_builtin_name
    UserType,     // 用户声明的类型名（含 Generic 的 std.result / std.option）
    TypeVar,      // 类型变量：T / K / V
    Constraint,   // 运算符接口 / 约束名：Add / Ord / ...
    Predicate,    // 编译期谓词：Terminates / Sorted / ...
}
```

**Key design point: determining `NameKind` requires exactly the information the parser currently
cannot obtain** (the type environment, declared operator-interface table, and declared predicate
table). This field therefore **cannot be determined independently inside the parser**—it must be
assigned by an injected environment. This connects directly to 5.4.

**Disposition of the synonym table (landed in WBS 6.1.2, 2026-10-27, final user decision)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) **is retained, with
reduced semantics**. Before the refactoring, it was the disambiguator that bridged two equivalent
AST representations; after `Type::Int(64)` is deleted, “Int” has only one representation path, and
it degenerates into the **sole name→type resolution table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language builtin names (the
  signature strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP list all reference it).
- The `DateTime` arm records a real fix (#338 related issue: the return value of `now()` could not
  be passed to `format_time`)—it is a **semantic alias**, not another capitalization alias of
  `Int`/`int`, so it is retained separately through P10 S11 (issue #513).

**Resulting disposition** (final metric = 15 builtin names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                               | Before                                          | Disposition                                                                                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 builtin names                                                                                                                                                                                                                                       | Mixed into 13 arms with aliases                 | **One canonical name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                            |
| Capitalization and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                        | **Delete** (22 spellings), **with no lexer alias table**—corpus measurements found 0 noncanonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                                   | Shared arm with `Void`                          | **Retain**—`()` is the spelling of the `Void` type (standard-library signature strings use `() -> Void`), not another type                                                                                 |
| `DateTime`                                                                                                                                                                                                                                             | `datetime` shared an arm with lowercase aliases | **Retain only the capitalized** temporary compatibility alias, with a note that its migration belongs to P10 S11 (issue #513)                                                                              |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares an arm
with `Void`; there is also a temporary `DateTime` alias). After deleting 22 spellings, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—**it is no longer silently accepted**.
Previously, `add_builtin_types` also registered lowercase copies of `int` / `float` / `bool` /
`string` / `void` / `never` / `char` in the type table, constituting a second alias table; those
were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive/negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected `MonoType`
   for each of the 15 names; the negative case already asserts
   `from_builtin_name("DateTime") == None`—`DateTime` no longer maps to `Int(64)`);
2. T2.5, the “deleted-spelling sentinel,” in `tools/type-tables --report`—if any of the 23 entries
   (the 22 spellings deleted under the 6.1.2 final decision + `DateTime` migrated out by S11.1) is
   added back to the resolution table, the report immediately records it as **unregistered drift**
   (there is no disposition to register: that would be a rollback).

**Consolidating the two `name_used_as_type*` implementations**: the two duplicate implementations
recorded in 2.4 must be merged into one; otherwise the semantic information supplied by `NameKind`
can be bypassed by two independent string-comparison implementations. The merged implementation
belongs in `parser/ast.rs` (next to the `Type` data), and `declarations.rs` calls it. The
`is_predicate` short-circuit reads `NameKind::Predicate`, and `Struct` / `NamedStruct` field
recursion follows the `ast.rs:894-905` version (the newer, more complete implementation).

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps with the full set handled
by `from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the check in
`extract_generic_param_names` (same file, measured 932) can become “`kind == Builtin` and the name
belongs to the accepted const-parameter set,” explicitly connecting the two sources of knowledge;
the const-parameter subset definition is validated by the T2 gate (see 5.7).

> **Design judgment**: if `.yx` corpus files contain type annotations such as `i64` / `int`,
> normalizing them to `Int` changes those files' **diagnostic positions** (but not diagnostic
> codes). C3 permits this (messages and ordering may be normalized), but the baseline must be
> recorded in Phase 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of `tests/yaoxiang` (316
> `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **0 noncanonical names in real type
> positions**; 16 raw-token matches were adjudicated individually as 12 strings + 4 comments (both
> `i64` / `f64` occurrences were in comments). Therefore 6.1's name normalization **does not change
> diagnostic positions in any corpus file**, and **no baseline regeneration is needed**.
>
> **Retest (P6 6.1 closeout, 2026-10-28)**: 6.1.1 rescanned the finalized **22** spellings. The
> 2026-10-27 measurement used a **narrow metric** (counting only the 14 integer/floating-point
> family spellings and excluding namespace use in `src/std`), and therefore recorded “16 raw-token
> matches = 12 strings + 4 comments.” The retest scope was 393 files: 316 `.yx` files in
> `tests/yaoxiang`, 39 in `tests/yaoxiang-multifile`, and 38 in `src/std`; it found **308 matching
> lines**. Only **2 lines were type-position candidates, and both were inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`'s `i64::MAX` and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`'s `-> f64`) ⇒ **real type positions
> still number 0**. All other matches were individually adjudicated as string literals / comments /
> the `std.string` module name and namespace access / a `Json` variant name / the `void` literal;
> **not one was a type annotation**. See the 6.1.1 line in [09](09-execution-wbs.md) and 09a “P6 6.1
> Name Normalization Closeout” for the retest command. The conclusion matches 6.0: 6.1's name
> normalization **does not change diagnostic positions in any corpus file**, and **no baseline
> regeneration is needed**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed within the parser + 2 out-of-layer references):

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

| File                                                                                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                     | Define `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is legitimate dependency inversion. **The landed method names are authoritative here** (the original design's `is_type_var` / `is_builtin_type` were not adopted: type-variable identity is determined structurally by the declaration site, not by name, so no such predicate is provided; builtin-name classification is instead supplied by `kind_of`, and `knows_builtin_types()` declares whether this probe holds the builtin type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                              | Retain `declare_predicate` / `is_predicate_name` (for current-pass accumulation); add a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**borrowed rather than `Box`**). **The landed state also adds a `kind_of()` accessor**—current-pass declared predicates take priority, and all others are delegated to the probe; this is the sole entry point for name classification                                                                                                                                                                                                                                                         |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                 | Delete `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                 | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/parser/ast.rs:930`                                                                         | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and should not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                        |
| All **40** parser construction sites (`ParserState::new` is the sole constructor; **4 production + 36 test**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. The **default implementation, `NullProbe` (all results false)**, keeps parser unit tests unaffected; the default production probe is `BuiltinProbe` (the compiler's builtin closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (The original design's “5 construction sites” was a pre-survey estimate; the measured value is **40**—`Parser::new` does not exist, and `ParserState::new` is the only constructor)                                                                                                                                              |

**Signature issue for `extract_generic_param_names`**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that receives no context. After removing the cross-layer call at `ast.rs:930`, it
still needs an `is_constraint` decision, so its signature must gain `probe: &dyn TypeEnvProbe`.
Callers (including `parser/tests/ast.rs`) are updated to pass `&NullProbe`.

**When `NameKind` is assigned**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to assign `kind`, fixing it once so that all subsequent match arms no longer perform
string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them concern parser-internal
functions under the 2.4 metric, while the genuine cross-layer references are the 2 sites in
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Consolidation of `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Position: delete `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This was already the **active path** (`pratt/nud.rs` constructs
`Expr::Lambda`, and `parser/statements/declarations.rs` constructs `StmtKind::Assign`; see 2.6); the
`FnDef` branch was a never-executed parallel implementation.

**Landed state**: the variant definition and all **15** production consuming arms were **deleted in
the same batch** (Rust's exhaustive `match` makes “definition + arm completion” an atomic unit; see
“Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5). Baseline consumption sites are classified
as follows:

| Category                                                                       | Count | Disposition                                                                                        |
| ------------------------------------------------------------------------------ | ----- | -------------------------------------------------------------------------------------------------- |
| Variant definition                                                             | 1     | Delete                                                                                             |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (otherwise compilation fails)                                                               |
| Non-exhaustive match arms + or-patterns with `_ => Span::dummy()`              | 10    | Delete (after deletion, they fall through to the fallback; retaining them leaves a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after deletion, they never match, with no error or warning)                                |
| Test construction sites                                                        | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not delete the cases)                                     |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally identical to the first three fields of `FnDef`; the `name` field unique to `FnDef` has
no usable location at the `Expr` layer (the name resides on `StmtKind::Assign.target`).

**Risk and mitigation (originally planned; now landed)**: those 2 non-exhaustive `if let` sites
become impossible to match after the refactoring—without errors or warnings. Mitigation is
**compile-time certainty**: after the variant is deleted, any new `if let Expr::FnDef` fails to
compile (the variant does not exist), so **no gate is needed**.

**Per-site disposition table, test migration, and measured results**: see
[09a](./09a-wbs-history.md) §P6 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**Delete `signature_params: Vec<Param>` from `StmtKind::Assign` / `StmtKind::TypeDefinition`.**

Rationale: its reason for existing (the original comment at `ast.rs:244`) was “for the typechecker's
`classify_generic_params`”—a **dedicated slot reserved by the statement AST for the type checker**,
redundantly transporting another representation of the parameters.

**The implementation metric differs from this section's initial draft in one important way**
(corrected after Phase 1 verification): the initial draft proposed “recompute `Lambda.params` in
place,” but `Lambda.params` is the **filtered** value-parameter list (constrained-position and
type-position parameters removed), whereas `classify_generic_params` / `extract_generic_param_names`
need the **unfiltered** original signature list. The actual implementation therefore moves the names
into the sole carrier first, then deletes the old field:

1. 6.8.1 first reshapes `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   co-reside, reusing the existing `Param`—no second structure and no parallel arrays);
2. 6.5.2 then deletes the two `signature_params` fields: `Assign` obtains them from
   `type_annotation`, while `TypeDefinition` switches to carrying the declared signature as
   `type_annotation: Option<Type>` (the same carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (curry groups are
   expanded in order, with `Paren`/`NamedParen` treated as transparent); `NamedParen`'s binder is
   not included and is interpreted explicitly by the type checker (see below).

> **Risk verification conclusion**: the initial draft's concern about the difference between
> “unfiltered” and “filtered” is valid—**but it is not a problem**: the flattened result is
> element-for-element sourced from the same `Param` nodes as the old `signature_params`, so
> generic-classification results do not change. The evidence is 356/356 corpus cases, zero drift in
> 216 IR snapshots, and a fully green `cargo test --all` (record in [09a](./09a-wbs-history.md) §P6
> 6.5). The initial draft's proposal to “use `Lambda.params`” would have changed
> generic-classification results and **was not adopted**.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, move binder semantics out (Completed:
6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records its **sufficient reason for existence** (see 2.7): if `r` is
lost from `(r: P(m))`, the type checker can only guess that “a free variable not in scope in a
constraint is the return-value formal parameter,” and will **silently substitute the undeclared `m`
as a binder as well** (the comment labels this a “measured defect”).

Therefore, **it cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param`/`param_span`
   carry source facts only and do not bear type-layer binder semantics.
2. Transfer binder interpretation to the **type-check layer**: when consuming `NamedParen`, the type
   checker **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node's
   field,” the same category as embedding `Expr` inside `ConstExpr`—the syntax node supplies facts,
   and the semantic layer performs interpretation.
3. Rewrite the comment from “the type checker can only guess” to “parameter-position formal names
   belong to `Type::Fn.params`; this node carries only the **return-position** binder.”

> **Path correction**: the `sema/` in this section's initial draft is the **target** directory name
> from RFC-039 D1; the semantic-layer directory in the current repository is
> `src/frontend/core/typecheck/` (the rename belongs to the 6.7 batch and has not yet run). All code
> paths in the body of this document use the pre-rename current paths (see “Directory Renaming and
> This Document's Paths”).

**Why this is a “migration,” not a “deletion”**: the syntax tree must represent source without loss.
`r` really exists in the source, and the AST cannot pretend otherwise. The problem is not the node's
existence but the **failure to assign responsibility for semantic interpretation explicitly**.

### 5.7 Build-Time Type-Table Gates

**The reference mechanism has already been verified in this repository**: `build.rs:19-37` invokes
`tools/code-tables`'s `parse_registry` / `validate`, performing uniqueness, segment, and
Chinese-completeness validation over 145 error codes; if `is_ok()` is false, it directly `panic!`s
and rejects the build. `build.rs:39-54` then compares each row against RFC-013's Markdown code table
and likewise `panic!`s on a mismatch. **RFC-013 documentation and code therefore remain consistently
aligned**—`tools/code-tables/Cargo.toml:15-16` has only a `serde_json` dependency, and the
validation logic is text-level parsing with no dependency on `syn`.

The type tables currently have **no equivalent gate**. This document adds a mechanism in the same
form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). The checks are as follows (the T3 column was deleted on 2026-10-30; its deletion
record is retained):

| #                                                                   | Check                                                                                                                                                                                                                                                                                                                                                                                                            | Inputs                                                    | Failure condition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Remedy                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site categories**                                 | Scan the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant's construction sites in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`). Reconstruction locations (②③) are driven by `scan.rs`'s `REBUILD_FUNCTIONS` table and reconciled line-by-line against source | Variant list + three-category hit table + survey baseline | ① A variant has **zero forward construction** and is not registered; ② a registered variant has a forward construction site; ③ a registered wildcard-arm fallback exemption does not match; ④ an **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered individually and must not be generalized to all wildcard arms); ⑤ **Survey baseline (T1.6) drift**: a variant's total/forward/AST→AST/reverse-bridge counts differ from `SCOUT_BASELINE` (decision 2A); ⑥ the reconstruction-location registry differs from source: a registered reconstruction function has no same-named `fn` in source (renamed/deleted without synchronization); ⑦ **metric and baseline are from different generations**: the current `REBUILD_FUNCTIONS` differs from the frozen metric snapshot `SCOUT_BASELINE_CALIBER` (metric revision) (6.6.3) | Delete the variant; or register a rationale / delete an invalid registration (see 5.2 and `REGISTERED_ZERO_FORWARD`); refresh `SCOUT_BASELINE` with reproducible evidence (including construction-site file:line); changing the metric requires refreshing the metric snapshot and baseline figures in the same batch; keep the registry synchronized with source |
| **T2 Synonym table and repository-wide name-list consistency**      | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `typecheck/probe.rs` (moved back from `parser/ast.rs` to L3 in WBS 6.6.9) + the LSP builtin type-name list at `src/lsp/world.rs:182`                                                                                                                                                                                             | Three tables                                              | A type name exists in one table but is not recognized by another (compared according to each table's semantic mapping rules); **from WBS 6.1.2, T2.5 is the “deleted-spelling sentinel”: adding back any of the 22 deleted spellings reports an error**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | `type-tables --fix` synchronizes in both directions                                                                                                                                                                                                                                                                                                               |
| ~~**T3 Builtin type names consistent with RFC-011 documentation**~~ | Originally planned: the canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                                                                                                         | —                                                         | **Deleted following the user ruling of 2026-10-30 (the comparison target does not exist)**: RFC-011 is a generic type-system design and its body contains no table whose column is “builtin type names”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | — (not applicable; see the explanation after the table for the deletion rationale and coverage)                                                                                                                                                                                                                                                                   |
| **T4 Exhaustiveness of `Type → MonoType`**                          | The `Type` variant list vs the match arms in `types/lower.rs`                                                                                                                                                                                                                                                                                                                                                    | Two lists                                                 | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Add an arm / delete an arm                                                                                                                                                                                                                                                                                                                                        |

> **T3's deletion and T2's coverage scope (user ruling, 2026-10-30)**: T3's **comparison target does
> not exist**—RFC-011 is a generic type-system design, and its body contains no table with “builtin
> type names” as a column. The initial implementation had to use “all tokens wrapped in inline code
> in all Markdown table rows whose entire token begins with an uppercase letter” as the
> documentation-side metric, which inevitably produced an empty-shell difference requiring no
> disposition (16 registered entries: 3 documentation-only + 13 code-only, none of which could be
> judged true or false). Therefore, the check, its 16 registrations, and the corresponding output
> sections in `--report` / `--json` were **deleted in the same batch**, with no placeholder left in
> code.
>
> **The only carriers of builtin type names are the three code-side lists, all of which are now
> reconciled name-by-name by T2**: ① `from_builtin_name` (`src/frontend/core/types/mono.rs`;
> acceptance surface = 15 canonical names, no aliases—the temporary `DateTime` compatibility alias
> was migrated out by S11.1 (issue #513), `--report` T2.1 measured “14 arms / 15 spellings,” with
> the first spelling in each arm canonical, the 22 spellings deleted from the implementation by
> 6.1.2 and the migrated `DateTime` guarded by the 23-entry T2.5 sentinel); ② `CONST_PARAM_TYPES`
> (`src/frontend/core/parser/ast.rs`; semantically a **subset** of ①, check ID C1); ③ the LSP
> builtin type-name list (`src/lsp/world.rs`; semantically a **superset** of ①'s canonical names,
> check IDs C2 / C3). No equivalent documentation-side list is established: the authoritative
> landing point for a new builtin name is ①, and T2 reports gaps in ②③ as **unregistered drift**.

**T4 implementation note**: Rust's exhaustive match already guarantees the “variant → arm” direction
at compile time. T4's value lies in the **reverse** direction—detecting stale arms in `lower.rs`
that point to deleted variants (this code already fails to compile when the variant is deleted, so
T4 is redundant). **T4 is therefore downgraded to a CI assertion** (a successful `cargo build`
passes it) and does not enter `build.rs`'s `panic!` path. **T1/T2 are the checks that genuinely
require text parsing** (T3 was deleted, as described above).

**T1's failure forms (6.6.2, user decision 2A, 2026-10-30)**: T1 has changed from “report only” to a
**hard build failure**—`build.rs` calls `type_tables::build_gate(root)`: `Ok(summary)` prints one
`cargo:warning` line (an informational line, not a violation), while `Err(details)` directly
`panic!`s and rejects the build. **All six violation categories use the same failure path** (see
items ①-⑤ and ⑦ above; ⑥ belongs to the scan-layer failure path “reconstruction-location registry
differs from source,” just like the different-generation metric in 6.6.3, and fails directly, but is
not counted as a gate-violation category). Each error identifies the **variant name** and the
construction-site / fallback-site / variant-definition `file:line`; baseline drift also includes
those locations (since 6.6.3 review F1: previously only figures were provided, leaving readers
without a place to look). Failure to complete the scan itself (file parse failure or incomplete
metric) also counts as failure—**it must not be downgraded to “pass.”** The registration mechanism
is retained with empty tables: `REGISTERED_ZERO_FORWARD` and `DEAD_VARIANTS_03` are now both empty
(after 6.3 deleted the 13 dead variants, no zero-construction variants remain). T1.7's “registered
but no longer valid / exemption did not match” alarms and injection-based validation
(`*_with(registry)`) continue to operate—any newly introduced zero-construction variant must be
deleted or have its rationale registered, or the build fails.

**Single source of truth for reconstruction locations (6.6.2, user decision 1A)**: categorization of
the second category / reverse bridges is driven by `scan.rs`'s `REBUILD_FUNCTIONS` table (**file +
function name → category**). `Type::without_fn_param_names` (the fidelity-preserving reconstruction
introduced in the 6.5 batch: recreates an existing AST type to strip type-position formal names; its
sole consumer normalizes the type before entering the IR through `ir_gen.rs`; **since 6.6.12 it was
deleted along with the IR-carrier narrowing**, and its registration and metric snapshot were removed
in the same batch) belongs to the **second category, AST→AST reconstruction**. Previously, it was
counted as parser-forward, inflating the forward count of 11 variants by 1 each (`Name` /
`ConstExpr` take the `self.clone()` path and produce no site). The registry is **reconciled
line-by-line with source**: if a registered function has no same-named `fn` in source, the scan
fails immediately and **must not silently fall back to forward classification** (otherwise a rename
or deletion would disguise reconstruction sites as construction sites).

**Baseline figures pin down two things (6.6.3 review F1/F2)**:

1. **The source tree**—`SCOUT_BASELINE_REV`: point counts are comparable only for the same source
   tree;
2. **The metric**—`SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` (the frozen snapshot of
   `scan.rs::REBUILD_FUNCTIONS` at that revision, recorded as “file + function → category key”). The
   snapshot deliberately **does not share a table with the registry**: sharing one would let both
   change together, making metric changes permanently invisible to the gate.

The gate performs machine-level reconciliation between the snapshot and the current registry
(`baseline::caliber_drift`, modeled after `scan::missing_rebuild_functions`): adding/removing a
reconstruction function or changing a function's category reports **“metric and baseline are from
different generations”** (including the metric revision and per-item differences), and **skips
baseline-count comparison**—counts across metrics are not comparable, and continuing to compare them
would disguise a metric change as numerical drift. The `--report` T1.6 section prints the baseline
revision and **metric revision**, so readers of the report or script runner can independently
recompute the counts under those revisions.

**Two implementation challenges for T1 and their mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and neither does `tools/code-tables`. Mitigation:
   - **Preferred**: add `syn` to `tools/type-tables` (only that crate; the root crate remains
     unchanged). `syn` is the standard approach, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing line-level heuristic in `tools/extract_arm.py` (an existing
     repository tool)—construction sites are on the right side of `=` or in function-argument
     position, while match arms are on the left side of `=>`. The number of `=>` occurrences is used
     as an approximation.
   - **Conservative starting point (completed in this order)**: when T1 first launched, it
     **reported only and did not fail** (`cargo:warning`), taking one full baseline to confirm that
     the heuristic was correct before changing it to `panic!`—since 6.0.1 it reports only; **6.6.2
     (2026-10-30) switched it to `panic!`**, and by then the baseline had already been refreshed
     under the same metric to 13/13 consistent rows. This follows the established approach of “a new
     CI gate may initially be red; establish a baseline first.”
2. **Reverse bridges and AST→AST reconstruction sites must be categorized separately**. 2.2 records
   two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and `function.rs:507-529`;
   2.2.2 records the AST→AST reconstruction sites in `function.rs`'s two substitution functions (15
   variants / 28 sites, including 5 variants / 10 sites from the 13-variant list). If T1 mixes them
   with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` and
   `Union` / `Option` / `Result` / `Sum` / `AssocType` will be judged as having construction sites
   and exempted from deletion, directly contradicting the disposition table in 5.2. T1's report must
   separate the three construction-site categories, and **fallback exemptions must apply only to
   individually registered entries** (currently only the `Void` parser error fallback at
   `types.rs:836`), never to all wildcard arms.

**Gate coverage (landed, 6.6.2, user decision 2A, 2026-10-30)**: `build.rs` calls
`type_tables::build_gate(root)` immediately after the `code-tables` validation block, using the same
`panic!`-rejects-build form—`Ok(summary)` prints one `cargo:warning`, while `Err(details)` invokes
`panic!("Type-table gate T1 failed (build rejected): …")`. **All six violation categories use the
same failure path**: unregistered zero-construction variant / registered but now forward-constructed
/ registered exemption did not match / unregistered wildcard-arm fallback site / **metric and
baseline from different generations** / **survey baseline (T1.6) drift**. Each error identifies the
variant and its construction site, fallback site, or variant-definition `file:line` (drift entries
also include construction-site locations since 6.6.3). Failure to complete the scan itself (such as
a file parse failure) also counts as failure and **must not be downgraded to “pass.”**

## Detailed Design

### 6.1 Type-System Impact

| Impact area                                                  | Assessment                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and never touches `ast::Type` variants                                                                                                                                                                                                                                                                                                          |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`) and `substitute` at `types/substitute.rs:112` / `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                                           |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `eval/reducer.rs:572`'s `unify` all operate on `MonoType`                                                                                                                                                                                                                   |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. Eight sites in `mono/function.rs` (`358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`) match `AstType::NamedStruct` / `AstType::AssocType`—`AssocType`'s disposition is deletion, and all 8 sites have been synchronized. **`function.rs:507-529`'s `mono_to_ast_type` was also changed** (see #47 in 6.3; it now uses `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match was consolidated with 6.3; the former `let params: Vec<MonoType>` directly reading `signature_params` now flattens through `Type::fn_params_flat` (the verified conclusion in 5.6: an unfiltered list sourced from the same elements as the old field)                                                                                  |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Requires changes**. `codegen/bytecode.rs:492-495`'s `type_id_to_monotype`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                                                                                                      |
| Interpreter (`backends/interpreter/`)                        | **Requires changes**. `image.rs:44`'s `type_table: Vec<ir::Type>` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects `{:?}` output                                                                                                                                                                                           |
| RFC-027 compile-time types                                   | **Requires changes**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                                                                                                           |

**IR-layer type discipline (to be inherited by 04-ssa)**: after consolidation, IR type annotations
may use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline should be
enforced by the “type consistency” invariant in 07's first validator layer. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are analogous expression-side forms—they may occur inside `ConstExpr`, and
therefore likewise must not descend into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** Basis:

1. The 13 deleted variants have **no construction sites in the parser-forward direction** (`Void`'s
   single forward site was an error fallback—**WBS 6.3 changed that fallback to `Type::Name{"()"}`
   and removed the gate exemption registration**, so there are now no forward construction sites) ⇒
   the parser does not produce them (the reverse bridge and AST→AST reconstruction dispositions are
   covered by #28 / #47 and the 10 AST→AST sites in 6.3.2, all implemented);
2. `ir::Type` is an alias of `ast::Type` ⇒ changing it back to `MonoType` does not change the set of
   types, only the **carrier**;
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs—this **may**
   change the behavior of some programs (types that were previously lost are now retained). **This
   is the sole direction of behavioral change, and it is a fix rather than a regression**, but 07's
   third-tier corpus differential must confirm that there is no regression.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After consolidation                       |
| ----------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`       |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match suffices) |

> **Design judgment**: because these 8 variants currently all collapse to `Void`, **every code path
> that depends on signature bytecode—the interpreter's arity/type checks and serialization
> format—currently sees only `Void`**. After consolidation, these locations will see the real types.
> **This is the only possible source of behavioral change in this document, and its direction is
> “restore correct information.”** If the corpus differential reports differences, they should be
> treated as **exposing an existing defect**, not as a regression introduced here—the verification
> method is to construct a `.yx` program containing a `Ref` parameter and compare `dump_bytecode`
> before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Category                          |
| --- | ---------------------------------------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`        | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**Completed: WBS 6.3.1, 2026-10-29**—all 13 variants deleted; `ast::Type` consolidated to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                     | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original location, **deleted**)      | Delete `Expr::FnDef` (**Completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consuming arms deleted in the same batch; zero references within `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                     | Delete `Assign.signature_params` (**Completed: WBS 6.5.2, 2026-10-30**—the same-named field was deleted from `TypeDefinition` in the same batch, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm, **deleted**)            | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | With #3 (completed)               |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                     | Change the `NamedParen` comment to “syntax supplies the binder name; semantic interpretation belongs to the **type-check layer**” (**Completed: WBS 6.5.2, 2026-10-30**—the comment now reads “parameter names belong to `Type::Fn.params`; this node carries only the return-position binder”; the `sema/` in the initial draft is D1's target directory name, while the current repository uses `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                        | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                               | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Addition                          |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                           | `trait TypeEnvProbe` + `NullProbe` (**Completed: WBS 6.4.1, 2026-10-29**—the landed state also has `BuiltinProbe` and `NameKind`; trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Addition                          |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                       | Add the `probe` field and accessors (**Completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, with `probe()` and `kind_of()` accessors; `ParserState::new` is the sole constructor, and **all 40 construction sites are wired**: 4 production + 36 test**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                     | Use `state.probe()` to assign `NameKind` (**Not done**: 6.4 introduced only the **classification enum + probe queries** and did not change the structure of `Type::Name`—fielding it would affect every production/test construction site and require the reverse bridge `mono_to_ast_type` to faithfully write back `kind`, forcing IR-snapshot and corpus-baseline regeneration, which conflicts with this batch's C3 surface (zero drift); fielding `NameKind` is deferred until a second consumer appears)                                                                                                                                                                                                                                                                                                                                           | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                     | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; only add an explanation of the variant deletion)—**Not done**: 6.4 did not change `types.rs` (its change surface was 6.3's fallback refactoring), and this line is deferred with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                         | Delete the `_ => (Vec::new(), Type::Void)` fallback and return an explicit error—**WBS 6.3.1 changed the fallback to a let-else explicit return of `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical). **The “return an explicit error” half was not done**—the original plan deferred it to 6.4's error-path refactoring, but **6.4 did not take over this path** (6.4 only implemented injection for type-name dependencies and did not change `types.rs`) ⇒ **6.5 did not take it over either** (that batch only reshaped representations and left the fallback path unchanged) ⇒ **it moved to 6.6**, where it is registered in 09's 6.6.1 line. The path is syntactically reachable (`semantic_tokens`' `Name` arm records one extra dummy-span token; 0 corpus hits, zero drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                     | Delete `n == "Terminates"`; use `state.probe().is_predicate(n)` (**Completed: WBS 6.4.3, 2026-10-29**—the landed implementation reads `NameKind::Predicate` through `ParserState::kind_of`: current-pass declared predicates take priority, and the rest are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                     | Delete the `typecheck::operator_interfaces::spec()` call (**Completed: WBS 6.4.3, 2026-10-29**—now uses `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                         | Delete the `signature_params` argument—**Completed (WBS 6.5.2, 2026-10-30)**: all construction sites deleted the field; in the same batch, the HashMap name table for the merged-name section was changed to positional alignment (criterion: preceding validation already guarantees element-by-element identical names)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | With #4                           |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                          | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | With #3 (completed)               |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                         | Retain (the parameter-list retrieval point)—**“Change to read Lambda directly as in 5.5” was not done**: 6.5.1's change list did not include this item, and its disposition is to be decided together with 6.8.2's declaration-driven checking mode                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`        | Delete 13 match arms (one per deleted variant) (**Completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | With #1                           |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                     | Remove synonyms (capitalization aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Consolidation                     |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                         | Retain the `DateTime` alias (**Completed: S11.1 (issue #513)—the alias was migrated out, and the bare name reports E1003/E1002**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                           | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Addition                          |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                                 | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Consolidation                     |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (current `7` is a private `use`) | Delete `pub use ... ast::Type` (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                         | Keep `FunctionCode.params` as `Vec<MonoType>` (already so)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                  | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Consolidation                     |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                         | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Consolidation                     |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                        | Change the `type_table` conversion to `.collect()` (elements already have the same type)—the actual change is to **delete the conversion and transport the values unchanged** (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #26                          |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)            | **Delete the entire `From<MonoType> for IrType` impl** (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                          | Change `type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Consolidation                     |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                         | Handle the `AstType::AssocType` arm according to the gate conclusion—**the conclusion was determined and implemented: WBS 6.3.2 deleted all 5 variants / 10 sites registered in 03 §2.2.2** (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each), and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                  | Type-table gates T1/T2 (the original T3 was deleted following the user ruling of 2026-10-30)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                         | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | With #3                           |
| 33  | `spawn/analysis.rs`                                                    | `912`                                         | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                          | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 35  | `orchestrator.rs`                                                      | `1492`                                        | Same, but obtain the name from `Assign.target`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | With #3                           |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                                | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                          | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                         | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | With #3                           |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                                 | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | With #3                           |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                     | Delete the `Expr::FnDef` test (**Completed: WBS 6.5.1, 2026-10-30**—the case was not deleted; it was rewritten as two Lambda-destructuring cases + one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                     | Rewrite as Lambda + Assign (**Completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`               | Delete lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                     | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be handled in the same batch as #1**—**the metric was corrected (Completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`); after 6.1.2 deletes lowercase spellings, it would become a `TypeRef` and **silently lose type identity**. The actual implementation therefore uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                      | Consolidation                     |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                  | Add `probe: &dyn TypeEnvProbe` to `extract_generic_param_names`'s signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` (**Completed: WBS 6.4.3, 2026-10-29**—the landed signature is at `ast.rs:927`; production caller `ir_gen.rs`'s 3 sites pass `SemanticProbe`, and the rest pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                    | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                       | Delete this file's `name_used_as_type` and call the consolidated version in `ast.rs`; the `is_predicate` short-circuit reads `NameKind::Predicate` (**Completed: WBS 6.4.4, 2026-10-29**—the consolidated single implementation is at `ast.rs:848` and covers the head check + predicate short-circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                         | Route the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3) (**Completed: WBS 6.4.5, 2026-10-29**—dual-knowledge wiring: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe without the builtin-name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Consolidation                     |

> **The 6 rows landed in 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed 2026-10-28)
> implemented these 6 rows in one batch—four carriers switched to `MonoType` (`bytecode.rs:814`,
> `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType` and the `ir::Type`
> re-export alias were deleted, and decoding was changed to transport values unchanged
> (`bytecode.rs:2315`, `:2316`, `:2346`). **These rows' line numbers have been refreshed to their
> landed positions** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390` were
> pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured results and gate
> outcomes.

> **The gate itself must be gated**: `tools/type-tables`, like `code-tables`, needs `mod`
> declaration wiring (the repository has already learned this lesson from 8 orphaned test trees).
> When creating the new crate, register it in the root `Cargo.toml` in the same batch.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/` (293 single-file `.yx` corpus files) + the multi-file corpus layer
established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library **must
all remain behaviorally unchanged** (identical diagnostic codes and messages; ordering may be
normalized).

| Risk                                                                          | Assessment                                                                                                                                                                                                                                                                                                    | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code uses type annotations such as `i64` / `int` / `f64`               | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                             | **Measured** (end of 5.3): 0 noncanonical names in real type positions ⇒ 6.1.2 directly deleted 22 spellings and **introduced no lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Source code uses `DateTime`                                                   | The temporary compatibility alias was migrated out of the builtin-name table by S11.1 (issue #513) (the alias arm was deleted; resolution behavior is at `mono.rs:1123` onward, with 15 names and no aliases), so it is not a builtin name                                                                    | The window is closed: before migration, `DateTime` was synonymous with `Int` and behavior did not change; after migration, a bare `DateTime` annotation becomes a `TypeRef` and reports E1003/E1002 (category-D semantic change, by user ruling)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `("()")` appears in type position                                             | It is not a product of the `Name` form but the spelling of `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                       | **Retain**: `()` shares an arm with `Void` and is one of the finalized 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `dump_bytecode` output changes                                                | The `type_table` element type changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                      | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only during C1/C2). However, the output diff must be recorded in Phase 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Behavior changes after the lossy bridge is fixed                              | See 6.2                                                                                                                                                                                                                                                                                                       | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| The result of type substitution changes after `mono_to_ast_type` is rewritten | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, using the `_` arm changes the result from something like `AstType::Int(64)` to `AstType::Name { name: "i64" }` | Phase 3 targeted comparison: select corpus files containing generic substitution and compare the monomorphized products' `type_name()` before and after the change. **Do not assume `{:?}` output is equivalent**. **Metric corrected (Phase 3 measurement)**: equality must not be judged using `type_name()`—it is the lowercase internal name used for diagnostic display, so comparing only `type_name()` can reveal that both are “int64” but not whether the canonical name is “Int64” or “i64”; instead, judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` are mutual inverses name-by-name) + structural comparison of the write-back product. WBS 6.3 measurement: the dumps and `.42` product MD5s of 4 generic-substitution programs are byte-for-byte identical to their pre-change versions |
| `.yx` source itself                                                           | **Zero modifications**                                                                                                                                                                                                                                                                                        | This document requires no changes to any corpus or std source file. This is a hard acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |

**Hard acceptance conditions** (**metric revised 2026-10-27, following the 2026-10-26 decision ②**):
`git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly driven test
files** under `tests/` may be modified when mechanically required by the change, but each
modification must be registered with **file / line count / rationale**, and **tests must not be
deleted or assertions weakened**. The original metric (`git diff --stat tests/ src/std/`) required
zero differences across all of `tests/`; this revision **relaxes the requirement only one level for
directly driven test files**. The zero-difference requirements for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`) remain
unchanged. **Exception**: the 6.8.2 behavior-fix batch is handled in a separate round with a
separate ruling.

## Implementation Essentials

The criterion uniformly uses 07's **C3: identical diagnostic codes and messages (ordering may be
normalized)**. The following is this workstream's phase breakdown; see RFC-039 for the global
cross-document ordering.

| Phase                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Rollback point                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2 (T3 landed in Phase 0 and was later deleted by the user ruling of 2026-10-30). The first T1 version **reports only and does not fail**. Take a full baseline against **unchanged** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | The T1 report accurately lists the 13 **parser-forward zero-construction** variants (including the classification of `AssocType`: 0 forward / 2 AST→AST, D8 unchanged), and **separately reports the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; 13 parser-forward zero-construction variants and 1 globally zero-construction variant (only `Enum`) are reported separately; T2 is fully green or supplies a repair list | No code changes; no rollback needed                                                                                                                                                                                                                                                                                                                                                                                             |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) consolidated to 15 names + the temporary `DateTime` alias; 22 in-implementation spellings deleted; `add_builtin_types`' 7 lowercase copies, the LSP list's `Uint` / `Int128`, and `sizeof`'s `Uint` arm cleaned up in the same batch; **no lexer alias table** (corpus measurement found 0 noncanonical names). **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**Verified: it uses the unfiltered original signature list; after 6.5.2 it is supplied by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                                                                        | C3: the full corpus diagnostic set is **identical item by item** to the Phase 0 baseline                                                                                                                                                                                                                                                                                                                                                                                      | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                                     |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers switched to `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy bridge `From<MonoType> for IrType` (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias were deleted (the `pub use` line at the head of `ir.rs` was demoted to a private `use`; that private `use` was subsequently deleted by WBS 6.6.12); decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), so the prior silent information loss in which 8 categories of `MonoType` variant collapsed to `Void` no longer occurs                                                                                                                                                                                          | C3 + targeted `dump_bytecode` comparison (item by item in the 6.2 table)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                             | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                                     |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: deleted 13 variants according to the 5.2 table (`ast::Type` 26 → 13); removed matching arms such as `mono.rs:699-708` and the `types.rs:836` fallback (changed to a let-else explicit return of `Type::Name{"()"}`); **handled `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; updated `types/tests/mono.rs`. **#47's metric was corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” became invalid after 6.1—`type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes lowercase spellings, it would become a `TypeRef` and **silently lose type identity**. The actual implementation uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back | C3 + fully green `cargo build` (exhaustive matches force every arm to be completed) + targeted generic-substitution comparison (last row of 6.4)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                     | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                                     |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: added `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); changed `parser_state.rs` (added the `probe` field and `probe()` / `kind_of()` accessors; **all 40 construction sites wired** = 4 production + 36 test); deleted the parser's 2 `operator_interfaces::spec()` calls and changed 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); consolidated the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connected `CONST_PARAM_TYPES` through the dual-knowledge `NameKind::Builtin` path (#50). **Not done**: adding a `kind` field to `Type::Name` at `types.rs:162-165` (#10; fielding conflicts with this batch's C3 surface), and changing the `types.rs:836` fallback to an explicit error return (#12, moved to 6.5)                         | C3 + **new assertions**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps (`typecheck` / `operator_interfaces`) measured 0 matches**; corpus 356/356, zero drift in 216 IR snapshots, `cargo test --all` exit 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                           | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                                     |
| **5. AST dead variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: deleted `Expr::FnDef` (baseline measured **25 sites / 16 files** = 1 definition + 15 production consumptions + 3 test constructions + 6 comments, of which 19 are code references) + deleted the parallel parameter tables from `Assign` / `TypeDefinition`; reshaped `Type::Fn.params` into `Vec<Param>` (names and types co-reside) ⇒ a single flattening entry point `Type::fn_params_flat`; retained `NamedParen` as a syntax node, with the binder interpreted explicitly by the type checker                                                                                                                                                                                                                                                                                                                                           | C3 + **22 modules** in `tests/integration/` fully green (22 `.rs` files measured; the original text's 18 was an old number); formatter parameter-position output byte-for-byte unchanged (355 `.yx` corpus files compared byte-for-byte); corpus 356/356, zero drift in 216 IR snapshots; `#[test]` count within `src/` 2878 → 2883 (no tests deleted)                                                                                                                        | Split into 7 commits (2 variant-deletion commits + 3 representation-reshaping/old-representation-removal commits + 2 fix-round commits); **revert policy**: the variant-deletion commit can be reverted independently, while the test-migration commit must be reverted in tandem (see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5); see [09a](./09a-wbs-history.md) §P6 6.5 for measured results and gate outcomes |
| **6. Gates become hard failures**             | **Completed (6.6.2, user decision 2A, 2026-10-30)**: T1 changed from warning to `panic!`, and **T1.6 survey-baseline reconciliation was connected to the same failure path** (a construction-site count mismatch with `SCOUT_BASELINE` fails); under decision 1A, `without_fn_param_names` was moved into the second category (restoring the frozen forward count); T4 was downgraded to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | `cargo build` **must fail** when a zero-construction variant is deliberately introduced; changing any `SCOUT_BASELINE` variant count to 1 **must also fail**—both mutations were tested with mechanically reversible patches (`scripts/dev/scratch.py trial`)                                                                                                                                                                                                                 | Separate commit                                                                                                                                                                                                                                                                                                                                                                                                                 |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow Phase 2**—before Phase
2 lands, the bytecode-side reverse bridge A (`bytecode.rs`'s `From<MonoType> for IrType`) consumes
`IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation. That
bridge was deleted in 6.2.2. **The remaining rewrite surface for Phase 3's variant deletion is
reverse bridge B** (`middle/passes/mono/function.rs`'s `mono_to_ast_type`, which still resolves
`String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`. Phase 3's #47
must be handled in the same batch as #1; otherwise `mono_to_ast_type` silently changes behavior.
**Phase 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B was changed to use
`canonical_name` + structural write-back; **the three `ast::Type` carriers in `middle/core/ir.rs`
were not changed**—6.3 only deleted variants and did not alter the IR shape (IR snapshot 216 and
corpus baseline 356 had zero drift), and the carrier disposition **actually landed in 6.6.11 /
6.6.12** (two zero-producer dead variants deleted + `TypeDecl` replaced with an IR-native narrowed
carrier; `ir.rs` no longer imports `ast`). The original “defer to Phase 4” schedule was not
fulfilled; this paragraph is authoritative.

**Downstream dependency**: only after all phases in this document are complete may
[04-ssa.md](04-ssa.md) begin. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all have the same element type, giving SSA type annotations a single landing point.

### Mandatory Verification for Each Phase

| Phase | Must execute                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction-site category**)                                                                                                                                                                                                                                                                                                                                            |
| 1-5   | `cargo test`; full corpus differential over `tests/yaoxiang/` (07's third tier); compare every C3 criterion                                                                                                                                                                                                                                                                                                                                                                                                    |
| 2     | Additionally: construct one `.yx` sample for every variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                                            |
| 3     | Additionally: for corpus files containing generic substitution, compare the monomorphized products' `type_name()` before and after the change—**the metric was corrected (Phase 3 measurement)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient to establish equality; instead, judge by **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` are mutual inverses name-by-name) + structural comparison of the write-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                                                |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                     | Rationale                                                                                                                                                                                                                                                                                                 |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                                                                                                                                      | The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) directly caused the mismatch between the field types of `BytecodeFunction.params` and `FunctionCode.params` (2.1). Once the alias is deleted, both field types are unified, and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                    | It would create an L2→L3 reverse dependency (violating RFC-039 layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See the full argument in 5.1                                                                                                                                         |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and move binder semantics to the **type-check layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is D1's target directory name, while the current repository directory is `typecheck/`) | `ast.rs:518-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as a binder                                                                                                                                        |
| Directory rename             | **Do it** (D1), as a pure relocation batch at the end of P5/P6; paths in this document's body use the current pre-rename paths                                                                                                                 | Decisions of 2026-10-03 (ChenXu233); supplementary implementation approach of 2026-10-04                                                                                                                                                                                                                  |

### Rejected Alternatives

None of the four alternative approaches was adopted: **retaining all three representations and
adding a conversion layer** resolves none of the root causes, and `ir::Type` remains non-independent
while dead variants, the synonym table, and reverse dependencies all persist; **using a macro to
generate the synonym table** treats a symptom rather than a cause—once `Type::Int(usize)` is
deleted, the macro has no reason to exist, and consistency should instead be guaranteed by gate T2
(which is more verifiable than the correctness of a macro expansion); **deleting only dead variants
without unification** has the lowest cost but leaves the problems in place, with no gate to prevent
the next person from adding another `Type::Int128(usize)` and growing the same dead-variant class
(if only one commit is possible, it should explicitly be recorded as a subset of this alternative,
with the T1 gate delivered alongside it); **unifying on `MonoType`** is addressed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. `Expr::FnDef`'s 15 production consumers
  (including 3 exhaustive arms) + 3 test constructions + 1 variant definition form a never-executed
  parallel path; deleting it removes a branch from each of `ir_gen.rs`, `ownership.rs`, and
  `termination.rs`.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  already differ in definition-body field recursion; after consolidation, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legitimate type value.”** This
  is the prerequisite for 07's first-layer “type consistency” invariant to pass genuinely.
- **The gate makes regression impossible**. T1 turns “adding a zero-construction variant” from a
  silent pass into a build failure—the repository's existing `build.rs` mechanism proves that this
  path is practical.
- **Zero source-code changes**. The 293 `.yx` corpus files and std library require no modifications,
  which is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may change behavior**. After the lossy bridge is fixed, 8 `MonoType` variants become
  visible as their real types rather than `Void`. The direction is a fix, but it changes paths that
  depend on signature bytecode. This is the document's largest uncertainty.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (such as counting `=>`) rather than `syn`, it may misclassify variants. This is why manual
  verification is mandatory in Phase 0.
- **If T1 misses reverse bridges, the disposition table and gate contradict each other**. 2.2 proves
  that 7 “dead variants” have production reconstruction sites in reverse bridges. If T1 does not
  separate construction-site directions, those 7 variants are judged as having construction sites
  and exempted from deletion—the gate becomes an obstacle to the deletion work rather than a
  safeguard. | **2. Deleting `signature_params` carries semantic risk** (**verified and resolved:
  6.5.2, 2026-10-30**). It carries the **unfiltered** parameter list, whereas `Lambda.params` is
  **filtered**. If `classify_generic_params` depends on the former, the refactoring would change
  generic-classification results. **Measured conclusion**: `classify_generic_params` /
  `extract_generic_param_names` need the unfiltered list; flattening `Type::Fn.params` is sourced
  from the same `Param` nodes as the old `signature_params`, so classification results do not
  change. The proposal to use `Lambda.params` would change behavior and **was not adopted** (corpus
  356/356, snapshots 216, `cargo test --all` fully green). | **3. Phase 3 touches
  `middle/passes/mono/function.rs`**. Besides the 8 `AssocType` matches, `507-529`'s
  `mono_to_ast_type` is the exit point for monomorphized type substitution, and rewriting it may
  change the substitution result's `type_name()` (last row of 6.4).
- **Noncanonical type names in the `.yx` corpus may be nonzero**. If the corpus extensively uses
  `i64` / `int`, Phase 1's “directly delete synonyms” approach is infeasible and would degrade to
  “move them into a lexer alias table,” reducing the consolidation achieved by `from_builtin_name`.
  (**Measured as 0**: the 6.0 survey in the end of 5.3 found none, so WBS 6.1.2 directly deleted the
  aliases and did not take the alias-table route.)
- **The gate may initially be red** (**this risk was handled in two steps**). If T1 launched
  directly with `panic!`, it would immediately block compilation; therefore, since 6.0.1 it reported
  first to establish a baseline, and only 6.6.2 switched it to `panic!`. By the time it became a
  hard failure, the baseline had already been refreshed under the same metric to 13/13 consistent
  rows; the current tree has 0 zero-construction variants.

### Open Questions

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1-D50) for the
> item-by-item decisions. **This document retains no pending items.**

## See Also

### Parent and Same-Batch Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — parent master plan; four-layer model, criterion tiers, routing table C (3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction rules, target
  directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; prerequisite and downstream consumer of this document
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; consolidation of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, first-layer IR validator's type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — generic mechanism
  and type-level constructors (`If` / `Match`); **does not carry a builtin type-name list**, so it
  is not a gate comparison target (the original T3 was deleted following the user ruling of
  2026-10-30; see 5.7)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — model for
  build-time gates
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line anchors were refreshed to the 6.5 landed state** (2026-10-30); entries removed by earlier
> batches are retained and labeled with their deletion batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after 6.3
  closeout; the 26-variant-era range was 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — evidence of the `Type::NamedParen` binder semantics
  and the defect its deletion would reintroduce
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types co-reside) and
  `Type::fn_params_flat` (flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- (**Deleted, WBS 6.6.12**) `src/frontend/core/parser/ast.rs`'s `Type::without_fn_param_names`:
  introduced by the 6.5 batch as a printing boundary while the IR used `ast::Type` as its carrier;
  after 6.6.12 replaced `FunctionBody::TypeDecl` with an IR-native narrowed carrier, the AST no
  longer enters the IR, the function lost its sole production call site, and it was deleted (T1's
  `REBUILD_FUNCTIONS` / `SCOUT_BASELINE_CALIBER` lost one entry in the same batch)
- `src/frontend/core/typecheck/probe.rs` (from WBS 6.6.9)—`CONST_PARAM_TYPES`: the sole
  authoritative list of **acceptable type names** for const generic parameters (originally in
  `parser/ast.rs`, the third copy of L3 knowledge; moved back to L3 when the parser's closed set was
  eliminated, with the parser asking only through `TypeEnvProbe::is_const_param_type`; see 5.4)
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single implementation after 6.4
  consolidation (the original `name_used_as_type_in` and the copy in `declarations.rs` have been
  merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through the injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: fallback for non-`Fn`
  forms (since 6.3, returns `Type::Name{"()"}`; the `Type::Void` variant was deleted)
- (**Deleted, 6.4**) `src/frontend/core/parser/statements/declarations.rs:42-80` — the original
  `name_used_as_type`, merged into the single implementation in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — the hardcoded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call (since 6.4 these use `ParserState::kind_of` / the
  injected probe; both literal references have disappeared, leaving only the comment)
- `src/frontend/core/parser/pratt/nud.rs:512` — expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — the `expr_to_params` arm retrieves the parameter
  list
- `src/frontend/core/types/mono.rs:1123` — `from_builtin_name`'s builtin-name resolution table (15
  names, no aliases—`DateTime` was migrated out by S11.1 (issue #513); WBS 6.1.2 final)
- `src/frontend/core/types/mono.rs:960` — `From<ast::Type> for MonoType` implementation (12 of the
  13 match arms are here)
- `src/frontend/core/types/mono.rs:169` — `MonoType` definition (approximately 22 variants)
- `src/frontend/core/types/const_data.rs:240`, `:327` — `const_data::BinOp` / `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- (**Deleted, 6.2.2**) `src/middle/core/ir.rs:3` — evidence that `ir::Type` was an alias of
  `ast::Type`
- `src/middle/core/bytecode.rs:809` (`BytecodeFunction`), `:814` / `:816` (`params` / `return_type`,
  carrying `MonoType` since 6.2)
- (**Deleted, 6.2.2**) `src/middle/core/bytecode.rs:2352-2390` — lossy `From<MonoType> for IrType`
  bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `:275-278` — `BytecodeFile::type_table` and
  `FunctionCode`'s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: the reverse bridge after the 6.3
  rewrite (using `canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — interpreter module's `type_table`
- `build.rs:33-51` — build-time error-code registry gate (model for the type-table gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
