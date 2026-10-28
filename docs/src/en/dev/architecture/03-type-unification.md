# Unification of Type Representations

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiering of acceptance criteria, and execution phase ordering are covered in the
> main RFC-039 text; the positioning of each companion document is covered in
> [this directory's index](index.md).

## Purpose and Scope

### Coverage

Consolidate the multiple parallel type representations in use today into a single representation,
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero forward construction in the parser** (within the 26 variants
  at `ast.rs:427-541`; see 2.2; the "forward" count does not subtract reverse-bridge or AST→AST
  reconstruction sites)
- **One handwritten type-name synonym table** (`mono.rs:839-886`; see 2.3; WBS 6.1.2 has already
  consolidated this into a 15-name resolution table)
- **The parser's actual reverse dependency on the typecheck layer** (2 locations; see 2.4)
- **One expression variant with zero production construction**, `Expr::FnDef` (`ast.rs:37-43`; see
  2.6)

The concrete deliverables are four items: a disposition table covering each of the 26 variants
(5.2), semantic classification of `Type::Name` through `NameKind` and elimination of the synonym
table (5.3), injected data flow through `TypeEnvProbe` (5.4), and generation-time type-table gates
T1–T3 (5.7).

### Directory Rename and This Document's Path

RFC-039 decision D1 determines that the directory rename (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) will be completed **together with P5/P6, without staging it
separately**. This document's 50-item change list and all acceptance greps are **written against the
current paths before the rename**—they are the construction baseline for each P6 batch **before the
rename batch**; the directory rename at the end of the batch is a **pure-move batch** (C1
zero-diff), committed separately. The baseline becoming invalid after the rename commit is expected;
use git history as the bisection basis.

### Out of Scope

- **How the directory rename is performed**. During this phase, `Type` does not move out of
  `parser/ast.rs`; only `lower.rs` is added under `types/` as the sole conversion point. The
  directory rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 in accordance with
  RFC-039 decision D1, in the form shown by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction specification**. See
  [01-routing.md](01-routing.md).
- **Tiering equivalence criteria and validator implementation**. The definitions of C1–C6 and the
  three tiers of criteria—IR validator, normalized snapshots, and corpus differential—are covered in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document references only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reasons
  described below.
- **The global P1–P10 execution sequence**. See the main RFC-039 text and
  [02-stage-contract.md](02-stage-contract.md). This document provides only the phase breakdown for
  its own workstream (see “Implementation Essentials”).

### Division of Responsibility with RFC-039

RFC-039 is the upper-level master plan: the four-layer model, criterion tiers, cross-document phase
ordering, and the rationale for performing this refactoring now. This document is the construction
plan for **the type-representation convergence item in RFC-039**: which variants to delete, which
files to change, in what order, and which gates prevent regressions.

All line numbers, variant lists, and construction-site counts in this document come from static
verification of `src/`, with each item citing `file:line`. **This document does not record the
execution process; it records code facts and the dispositions derived from them.**

To facilitate cross-references with documents in the same batch, the “Target Design” and “Detailed
Design” chapters retain the `5.x` / `6.x` subsection numbering used elsewhere; the remaining
chapters follow this directory's conventions.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(`ir.rs:3`). [04-ssa.md](04-ssa.md) intends to introduce register type annotations and new
instructions into the IR. If the representations are not consolidated beforehand, SSA type
annotations will grow directly on the syntactic type's 26 variants through the `ir::Type` alias,
effectively institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree inside a type.
  It is impossible for an `Expr` to appear in an IR instruction's type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations over 26 variants would permanently cement the 13 dead variants and the
two syntax-specific variants into the IR contract.

## Current State

Everything below is a statement of code fact and contains no proposals. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and Lossy Bridging

> **Status note (2026-10-28; WBS 6.2 has landed)**: this section records the state **before** WBS
> 6.2 landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) has changed four type carriers to carry `MonoType`,
> removed the lossy `From<MonoType> for IrType` bridge, and removed the `ir::Type` re-export alias.
> All statements below involving `ir::Type` or the lossy bridge are **pre-change facts**; locations
> marked “(deleted in 6.2.2)” reflect an implemented disposition. Implementation details and
> measured results are covered in [09a](./09a-wbs-history.md) §P6 6.2.

The multiplicity of type representations is the common upstream source of four independent defects:

| Downstream symptom                                                                   | Causal relationship to type representation                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                      | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, the parser chooses the latter, and typecheck must supply a table to translate the former back                                                                           |
| Bytecode-layer signatures differ from IR-layer signatures                            | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Thus `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places without any checking | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match at `bytecode.rs:2371-2376`; the same mapping is written in both places, with no cross-checking                                                                                                                |
| The parser hard-codes `"Terminates"`                                                 | Because the parser cannot determine whether something is a type or an operator-interface name, it can only embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                               |

**The first three are representation issues; the fourth is a downstream side effect of a
representation issue.** Implementing only item 1 (deleting dead variants) would leave the synonym
table and reverse dependency in place. Implementing only item 4 (injecting a callback into the
parser) would build on top of the still-inconsistent representations.

One of the three parallel representations is an alias illusion:

| Name        | Definition location                       | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, and `ir.rs:6` then uses
`crate::frontend::core::typecheck::MonoType`. The IR module depends on two “type” symbols at once,
but one of them is only an alias.

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

All eight `MonoType` variants collapsed into `IrType::Void`, in addition to a `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`), whose comment explicitly describes it as a “transitional branch
before deleting enum variants.”

Here, `IrType::Void` is a **legal type value, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function's return type was lost
during bridging”—one reason why 07's first-layer validator cannot truly pass its “type consistency”
invariant against the current code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants addressed in 2.2—**both
ends of the lossy bridge point to dead code**.

**Current state (from 6.2.2 onward)**: the impl has been **removed entirely**, and decoding no
longer degrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now transports the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those
eight `MonoType` variants retain their actual types in signatures and type tables, and
`IrType::Void` is no longer used as a substitute for “lost information.” The `ir::Type` re-export
alias (`src/middle/core/ir.rs:3`, `pub use ... ast::Type`) has also been reduced to a private import
(`:7`)—the IR layer no longer publicly re-exports the syntax-layer type. However, three IR-layer
carriers still directly hold `ast::Type` (`Instruction::Cast.target_type` /
`Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`), to be handled in 6.3.

### 2.2 Parser-Forward Variants with Zero Construction

**Counting scope**: construction-site scanning covers all of `src/`, excluding `tests/` and
`*/tests/*`. **A construction site is a `Type::<variant>` path in expression position** (pattern
positions are not counted), and sites are reported in **three separate categories that must not be
combined**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two substitution
functions in `middle/passes/mono/function.rs`); ③ **reverse bridging** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero construction in the parser-forward
direction** (reconstruction sites in the latter two categories are covered in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction site                                                                                                                                                                                                                                                                             |
| ------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                          |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                          |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                          |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                          |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                          |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an exemption item by item; removed with 6.4 #12)                                                                                                                                                   |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                          |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                               |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Definition-line-anchor drift record (measured in P6 6.0, 2026-10-27)**: the “Definition” column
> in this table and in 5.2 was written on 2026-10-05. Since then, 17 lines were added before the
> `ast::Type` enum (`pub enum Type` moved from 427 to 444), so **the definition lines of all 26
> variants uniformly shift by +17** (for example, `Int` moves from 432 → 449 and `NamedParen` from
> 533-540 → 550-557). This round only **records** the drift and does not change the table body; all
> later references use the `ast::Type` symbol as the anchor.

The root-cause chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… They use the
   same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site of `Type::Void` is an error fallback**—the
   `_ => (Vec::new(), Type::Void)` at `types.rs:836`, at the end of a match returning
   `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants are constructed in test code** (`src/frontend/core/types/tests/mono.rs:27-40`,
`97`, `119-145`, `221`; `src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`,
`525`, `567`, `591`, `620`). This document therefore uses the term “**parser-forward zero
construction**”: these are neither variants with “zero global references” nor variants with “zero
production construction”—11 of them do have production construction sites in one of the other two
categories (reverse bridging / AST→AST reconstruction). These tests themselves cement the behavior
under test: they assert exactly what these syntactic forms lower into as `MonoType`, even though the
parser never produces the forms. When deleting the variants, these tests must be deleted or
rewritten accordingly.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The “zero forward construction” designation above holds only in the **parser → AST
forward-construction direction**. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct seven variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant list                                                                             |
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

**This scope boundary directly affects the disposition**:

- The `bytecode.rs` location disappears with 6.3's #28 (deleting the entire
  `From<MonoType> for IrType` impl), so it adds no extra cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name reverse resolver (annotated `#299`), converting `String` / `Bytes` from `Generic` back
  into dedicated variants by name. After `ast::Type::String` / `Bytes` are deleted, these two guard
  branches must be rewritten. After `Int` / `Float` / `Bool` / `Char` / `Void` are deleted, the
  behavior of the `_` arm changes (some types that previously fell into `_` silently switch to
  `AstType::Name`). This is the 47th supplementary item for 6.3.
- **Direct consequence for the gate**: a T1 gate that scans only forward construction sites would
  classify the seven variants above as “having construction sites” and exclude them from the
  deletion list. T1 must explicitly include reverse bridges in its counting scope; otherwise the
  gate's conclusions will contradict this document's disposition table (see 5.7).

#### 2.2.2 Scope Boundary (2): AST→AST Reconstruction (the third category added in this revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` are **AST→AST** type-variable substitutions that reconstruct an
existing AST type; they are **not** `MonoType → AST` conversions. The P6 T1 gate measured the
following on the 6.0.2 baseline (2026-10-27): **15 variants / 28 sites**, of which **5 variants / 10
sites** belong to the 13-variant list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these five variants, it must rewrite all 10 sites in the same batch** (already
recorded in 09's 6.3.2); otherwise, these arms will fail to compile after the variants are deleted.

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

**The only reason this table exists is to bridge the AST layer's inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but structurally
different at the AST layer, and the parser only produces the latter, `from_builtin_name` must accept
case variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"` on the
`Void` line is especially revealing—it works with the `Type::Void` fallback at `types.rs:836`.

The same knowledge—“which names are built-in type names”—exists in seven copies throughout the
repository, with no consistency checking whatsoever:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, so the original line numbers have drifted)             | Form                                                                                                                                                                                                                        |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                             |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, copied `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                                     |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                    |
| 4   | `src/lsp/world.rs:182`                                                                                                            | Built-in type-name list on the LSP side (14 entries = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                   |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` aliases (registered by the P0 gate on 2026-10-06)                                                 |
| 6   | The base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                       | String match, name → byte size. It previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports Unknown type. **Dead arm: living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts typecheck's “Set except by name” behavior (`inference/expressions.rs:2917-2918`, #300 decision 4)                                               |

> **Line-anchor drift record (measured in P6 6.0, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223 and the others by 2–9 lines each), but
> **the number of entries and their contents have not changed** (item 1 had 13 arms and item 3 had
> 15 entries when this section was written; item 3 had 15 entries and item 4 had 15 entries then;
> after WBS 6.1.2, item 1 has 15 arms / 16 spellings and item 4 has 14 entries). All later
> references use **symbol anchors + repository paths**; this table's line numbers are for location
> only. The arm count in item 1 and the living drift evidence in items 5 / 6 / 7 (the `Uint` dead
> arm and the contradiction involving `Set`) **still exist**; this section has not changed code.

`CONST_PARAM_TYPES` is used by `extract_generic_param_names` (`src/frontend/core/parser/ast.rs`,
measured 932) to determine const generic parameters. It enumerates type names that may be used as
const parameters and has a **partially overlapping but different relationship** to the full set
handled by `from_builtin_name`, with no mechanism ensuring synchronization. Item 4 is the fourth
copy used for LSP syntax highlighting and hover, likewise without validation; T2's comparison scope
includes it.

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2025-10-06: item 5 is a second type-name synonym table; items 6 and 7 share the same root cause as
items 1–4 (built-in name → semantic / ID string-key tables). Their purposes differ, but their drift
risks are the same. The same gate also found 11 string dispatches for built-in container names on
the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4, etc.).
**These are not covered by this section or by 5.4's parser data flow**; the inventory and
disposition are recorded in [06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as a P6 extension
verification item.

### 2.4 The Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both functions belonging to the type layer).
**Measurement differs from this description**: both functions are defined inside the parser, not in
the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository's only `fn`
  definition)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the repository's only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are only **2 locations** in production code that genuinely cross layers into `typecheck`, and
both call the same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                                |
| --- | --------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Determines constrained-position parameters (`T: Add`), which do not occupy runtime parameter positions |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Determines constrained-position parameters and produces `GenericParamName` with `constraints`          |

The second location is deeper—**`parser/ast.rs` itself** (the entire `ast.rs:847-948` block and
`StmtKind` / `Expr` in the same file) directly references a typecheck symbol. Routing table C
records only one location, in `declarations.rs`.

Together with the hard-coding at `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Extended as more built-in predicates are added (the future open-set approach:
// pass predicate names down to the parser, or use a form determinable by the syntax layer).
// The parser does not hold the type environment, so it checks two sources: hard-coded
// built-in predicates plus declarations accumulated in this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment explicitly acknowledges this as a temporary form for a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is a downstream issue of type representation**: the parser needs to ask whether something
is an operator-interface name because it must distinguish whether `Type::Name` denotes a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings** (concrete type / type variable / constraint name / predicate name). The syntax
layer cannot determine this on its own, so the decision authority is forced upward into the type
layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` explicitly acknowledges
this:

```rust
/// Whether a parameter name is treated as a type reference in a given type
/// (`N` in `(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; implemented
/// independently here to avoid a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` in `declarations.rs:42-80`                                       | `name_used_as_type_in` in `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                              | No callback                                          |
| `Type::Generic` branch  | First short-circuits through `is_predicate(app_name)` and returns `false` (`:54-56`) | No short circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                                 | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                           | **Present** (`:880-882`, traverses `fields`)         |

The comments at `ast.rs:868-870` record the origin of the latter two branches: “These two branches
were previously missing, so when a const generic parameter was referenced inside a definition body,
it could not be determined that it was ‘used as a type’; annotation validation would incorrectly
report an unknown name for a valid const parameter.”

In other words, **the function was duplicated to avoid a cross-module dependency, but its fixes were
never merged back into the original**. The `ast.rs` version recursively traverses fields in a
definition body, while the `declarations.rs` version does not—the same source produces different
results at two decision points. This duplicate must be merged when `NameKind` is introduced in 5.3.

### 2.5 Parallel Operator Enums

There are two operator enums, neither of which references the other, and **their members have
already diverged**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                  |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Both also independently implement complete `Display` implementations (`const_data.rs:291-317`) and
classification predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at
`const_data.rs:260-289`).

**Why this is part of the type-representation issue**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in type-position expressions,
they fall into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly
related to 6.1's IR-layer type discipline: after convergence, `Type::ConstExpr` disappears following
`Type → MonoType` and is never visible to the IR, while `Assign` / `Deref` continue to exist through
paths such as `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
Because the two enums have diverged, neither side alone can determine which operators may appear in
type positions.

This phase does not merge the two enums (their ownership is covered under “changing the operator
change surface” in [05-frontend-paradigm.md](05-frontend-paradigm.md)). However, the T1 gate's scan
target should include `ast::BinOp` / `ast::UnOp`, so that “new zero-construction operators” become
equally visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction

It is true that `Expr::FnDef` (`ast.rs:37-43`) has **zero production construction**—the repository
contains only two construction sites: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

However, **there are far more consumption sites than four**. The measured production consumption
count is 12:

| File                                                     | Line           |
| -------------------------------------------------------- | -------------- |
| `src/frontend/core/spawn/placement.rs`                   | `122`          |
| `src/frontend/core/spawn/analysis.rs`                    | `912`          |
| `src/formatter/handlers/expr.rs`                         | `43`           |
| `src/frontend/module/orchestrator.rs`                    | `1492`         |
| `src/middle/core/ir_gen.rs`                              | `4325`, `5097` |
| `src/frontend/core/typecheck/checker.rs`                 | `1642`         |
| `src/frontend/core/typecheck/checker/semantic_tokens.rs` | `1416`         |
| `src/frontend/core/typecheck/inference/expressions.rs`   | `3374`         |
| `src/frontend/core/typecheck/inference/existential.rs`   | `55`           |
| `src/frontend/core/typecheck/passes/dead_code.rs`        | `305`          |
| `src/frontend/core/typecheck/layers/ownership.rs`        | `1207`         |
| `src/frontend/core/typecheck/layers/termination.rs`      | `752`          |

There are also 2 exhaustive match arms that exist solely for it: `ast.rs:1026` (`Expr::span()`) and
`pratt/mod.rs:47` (`expr_end_line`).

**The path actually taken by function definitions** is: `nud.rs:425-429` constructs `Expr::Lambda`,
and `declarations.rs:462-479` constructs `StmtKind::Assign` (placing the `Lambda` in `value`). In
other words, the `FnDef` branch is **a parallel path that never executes but is maintained at 14
locations**.

### 2.7 Syntax Nodes Storing Semantics for the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and the comment states
directly:

> `/// The first parameter group of the signature is preserved as-is (including parameter names), for the typechecker to use in classify_generic_params`

The statement AST reserves a dedicated slot for the type checker. Construction site:
`declarations.rs:472`.

**Second, expression positions are treated as parameter lists.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a Lambda with an
**empty body**. The arm at `src/frontend/core/parser/pratt/led.rs:466`,
`Expr::Lambda { params, .. } => Some(params.clone())`, then “retrieves” it and treats it as a
parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module's match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comments at
`ast.rs:519-540` explicitly state:

> RFC-027 §3's return-position refinement relies on it to declare the **return formal parameter
> name** (called the binder here)… **Without it, the type checker can only guess** that “a free
> variable not in scope in a constraint is the return formal parameter,” so `(r: P(m))` silently
> substitutes the undeclared `m` as a binder as well (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Position: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is downgraded to the type checker's internal working representation; the `ir::Type` alias
is deleted.**

The three representations converge to **two roles**, but only one is a “type”:

```
The sole type structure  Type  ─────────────────────────────┐
   (26 → 13 variants, lives in types/ rather than parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, no semantic loss)
                                                  │  ← sole conversion point types/lower.rs
Type-checking working representation  MonoType  ──────────────────────┘
   (carries TypeVar / substitution state, flows only inside typecheck)
   ×
ir::Type alias deleted, BytecodeFunction.params / type_table switched to MonoType
```

**Why not “merge into `MonoType`”** (this is the proposal most likely to be raised, and is rejected
here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.);
   `MonoType` does not. 07's C3 criterion is “same diagnostic codes and messages”—diagnostic
   locations must remain traceable to source. Adding `Span` to `MonoType` would burden monomorphized
   types with a collection of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, and
   `pub fn unify` at `types/solver.rs:411`). These are **intermediate states of the type checker's
   solving process**, not types themselves. Putting them into the sole type structure would require
   the IR construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) and are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counterargument**: promoting `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer's type. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this design's goals, so choosing a
   solution that introduces a reverse dependency is self-contradictory.

**Why `ir::Type` must be deleted rather than “retained”**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType` at
`codegen/bytecode.rs:277-278`. The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The only reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed in full,
eliminating the reverse reconstruction recorded in 2.2.

**Explicitly not done**: `Type` is not moved from `parser/ast.rs` to `sema/types/` during this
phase. This phase only adds `lower.rs` under `types/` as the sole conversion point, leaving the
definition of `Type` unchanged. The directory rename is performed as an independent move batch at
the end of P6 in accordance with D1—at that point, `Type`'s home is the top-level `ast/type_.rs`
(the AST top-level domain), while `sema/types/` is the home of the `MonoType` working
representation.

### 5.2 Disposition Table for All 26 Variants

> Disposition categories: **Delete** (zero forward construction in the parser), **Retain** (has
> forward construction), **Migrate** (semantics move to another node).

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain (strengthened)**                  | The parser's **sole** primitive-type exit (`types.rs:162-165`). After the 12 primitive-type variants are deleted, all primitive types use it. A semantic classification of the name must be added (see 5.3)                                                                                                                                                                                                                                   |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:699` is a match arm). The bit width is carried by the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                    |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                        |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                        |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:702` is a match arm). **Note**: `IrType::String` at `bytecode.rs:2372` consumes it, so that arm must be rewritten after deletion                                                                                                                                                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:703-706` are match arms). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                          |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                        |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The parser's only forward construction site is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an exemption item by item). The fallback is removed with 6.4's error-path refactoring                                                                                                                                                                                                                        |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                             |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has a production construction site at `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                          |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                 |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:747` is a match arm); the total across all three categories is 0—the only globally zero-construction variant. Enums use `Struct` + `TypeBodyItem`; this is unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                                                                |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed by `bytecode.rs:2366`; **field reshaping `params: Vec<Type>` → `Vec<Param>` (name and type colocated) follows in 6.8.1 (D54)**—the names of `Assign.signature_params` are thereby carried forward (see 6.5.2)                                                                                                                                                                                                                        |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that the `Generic` path is used                                                                                                                                                                                                |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                     |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                    |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero forward construction in the parser (the syntax layer has no `::` path that produces it); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (recorded in 6.3.2). **D8 unchanged**: the criterion is forward construction; if associated-type syntax is enabled in the future, a new proposal will redefine it |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero forward construction in the parser (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                     |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has a production construction site at `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                  |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type inside unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                         |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites at `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                        |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has a production construction site at `types.rs:160`. It is the only `Type` variant embedding `Expr`, and the IR layer is forbidden from carrying it (see 6.1)                                                                                                                                                                                                                                                                                |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004's curry layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                      |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | The semantics (binder name) move to the type checker; the AST retains only syntax. See 5.6                                                                                                                                                                                                                                                                                                                                                    |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs seven items from items 2–8
> in this table. In addition, the AST→AST reconstruction recorded in 2.2.2 covers 5 variants / 10
> sites within the 13-variant list (2 each for Union/Option/Result/Sum/AssocType). Both categories
> of reconstruction sites must be handled in the same batch as 6.3.1; otherwise, behavior either
> changes silently (see 6.3 #47) or the code no longer compiles.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly in accordance
> with D8 (item 18 in the 5.2 table). The **criterion is parser-forward construction**: zero forward
> construction holds because the syntax layer has no path that produces `::` associated types, so
> there is no need to change D8 or add it to the whitelist. The **2 production AST→AST
> reconstruction sites** (`function.rs:456`'s `substitute_type_ast` and `function.rs:623`'s
> `substitute_type_in_ast`) are already recorded in 6.3.2 and rewritten in the same batch as 6.3.
> T1's gate (5.7) has reproduced this conclusion through its three-category breakdown (all 26
> variants consistent in the measured 6.0.1 commit).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine whether something is a
predicate; lines `510-512` in the same file must determine whether something is an operator
interface; `src/frontend/core/parser/ast.rs:971` must determine whether something is an operator
interface; and the two `name_used_as_type*` implementations at `name_used_as_type_in` in the same
file (measured at 864) and `name_used_as_type` in `declarations.rs:42` must determine whether
something is a type reference. All of these rely on string comparison.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target form (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → passed to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: classifying `NameKind` requires precisely the information the parser cannot
currently obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be classified independently inside the parser**—it
must be determined by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 has landed; finalized by the user on 2026-10-27)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) is **retained, with its
role downgraded**. Before the change, it disambiguated two equivalent AST representations. After
`Type::Int(64)` is deleted, “Int” has only one representation path, and it degenerates into the
**sole name-to-type resolution table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language built-ins
  (`src/std/*.rs` signature strings, `CONST_PARAM_TYPES`, and the LSP list all use it as their
  reference).
- The `DateTime` arm records a real fix (#338 and related issue: `now()`'s return value could not be
  passed to `format_time`)—it is a **semantic alias**, not the same kind of case alias as
  `Int`/`int`, so it is retained separately through P10 S11 (issue #513).

**Resulting disposition** (final scope = 15 built-in names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                     | Before                                 | Disposition                                                                                                                                                                                                |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Mixed into 13 arms with aliases        | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                      |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                               | **Deleted** (22 spellings), **with no lexer alias table**—corpus measurement found 0 noncanonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | Shared arm with `Void`                 | **Retained**—`()` is a spelling of the `Void` type (standard-library signature strings write `() -> Void`), not another type                                                                               |
| `DateTime`                                                                                                                                                                                                                                   | Shared an arm with the lowercase alias | **Retain only the capitalized** temporary compatibility alias, with a note that its migration belongs to P10 S11 (issue #513)                                                                              |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares the
`Void` arm; there is also the temporary `DateTime` alias). After the 22 spellings are deleted, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—**it is no longer silently accepted**.
Previously, `add_builtin_types` also registered lowercase copies of `int` / `float` / `bool` /
`string` / `void` / `never` / `char` in the type table, effectively creating a second alias table;
these were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for all 22 spellings, and `DateTime` → `Int(64)`);
2. T2.5, the “deleted-spelling sentinel,” in `tools/type-tables --report`—if any of the 22 spellings
   is added back to the resolution table, the report immediately counts it as **unregistered drift**
   (there is no disposition that can be registered for it: that would be a rollback).

**Merging the two `name_used_as_type*` implementations**: the two duplicate implementations recorded
in 2.4 must be merged, otherwise the classification information provided by `NameKind` will be
bypassed by two independent string-comparison implementations. The merged implementation is placed
in `parser/ast.rs` (next to the `Type` data source), and `declarations.rs` calls it. The
`is_predicate` short circuit now reads `NameKind::Predicate`, and `Struct` / `NamedStruct` field
recursion follows the newer, more complete implementation at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter-name table in `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps the full set handled by
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured 932) can be changed to “`kind == Builtin` and the
name is in the acceptable const-parameter set,” explicitly connecting the two sources of knowledge.
The definition of the const-parameter subset is validated by the T2 gate (see 5.7).

> **Design judgment**: if the `.yx` corpus contains type annotations such as `i64` / `int`,
> normalizing them to `Int` changes those files' **diagnostic locations** (but not diagnostic
> codes). The C3 criterion permits this (messages and ordering may be normalized), but a baseline
> must be recorded in phase 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of `tests/yaoxiang`
> (316 `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **0 noncanonical names in actual
> type positions**; the 16 raw token matches were adjudicated individually as 12 strings + 4
> comments (both `i64` / `f64` occurrences were in comments). Therefore, 6.1's name normalization
> **does not change diagnostic locations in any corpus**, and **no baseline needs to be
> regenerated**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned using the finalized **22** spellings. The
> 2026-10-27 result used a **narrow scope** (counting only the 14 integer/floating-point-family
> spellings and excluding namespace usage in `src/std`), which is why it recorded “16 raw token
> matches = 12 strings + 4 comments.” The retest scope was `tests/yaoxiang` (316 `.yx` files) +
> `tests/yaoxiang-multifile` (39) + `src/std` (38) = **393 files**, with matches on **308 lines**.
> There were only **2 type-position candidates, both inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`, `i64::MAX`, and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`, `-> f64`) ⇒ **still 0 actual type
> positions**. All other matches were individually adjudicated as string literals / comments /
> `std.string` module names and namespace accesses / `Json` variant names / `void` literals, with
> **not one being a type annotation**. See 6.1.1's line in [09](09-execution-wbs.md) and 09a's “P6
> 6.1 Name Normalization Closure” for the retest command. The conclusion is consistent with 6.0:
> 6.1's name normalization **does not change diagnostic locations in any corpus**, and **no baseline
> needs to be regenerated**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (a closed loop inside the parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46 (owned by the parser; declarations parsed only in the current pass)
  ├─ n == "Terminates"        ← hard-coded literal (declarations.rs:499)
  └─ (the boundary-crossing call above)
```

**Target data flow** (injection, with no boundary crossing):

```
L1 orchestration layer (constructed once at the entry point)
   │  ① Compile the built-in operator-interface table
   │  ② Compile std's .yx declarations → predicate-name set
   ▼
TypeEnvProbe (trait, defined in L2, src/frontend/core/parser/probe.rs)
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState (extended at parser_state.rs:46/54 to delegate to the probe)
   ▼
declarations.rs:498-511 and ast.rs:930 changed to
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Changes by file**:

| File                                                                                | Change                                                                                                                                                                                                                                                  |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                           | Defines `trait TypeEnvProbe`, providing four pure query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2, with the implementation supplied by L3**—this is legitimate dependency inversion   |
| `src/frontend/core/parser/parser_state.rs:46-58`                                    | Retains `declare_predicate` / `is_predicate_name` (for accumulation during the current pass) and adds a `probe: Box<dyn TypeEnvProbe>` field and a `probe()` accessor                                                                                   |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                       | Deletes `n == "Terminates"`; changes it to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                              |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                       | Deletes the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; changes it to `state.probe().is_constraint(n)`                                                                                                                       |
| `src/frontend/core/parser/ast.rs:930`                                               | Deletes the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and should not hold an environment dependency—this function must accept a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction sites (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **The default implementation `NullProbe` (all queries return false)** ensures parser unit tests remain unaffected                                                                                    |

**The signature problem in `extract_generic_param_names`**: `ast.rs:891`'s
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` is a free function
that accepts no context. After the boundary-crossing call at `ast.rs:930` is removed, it needs an
`is_constraint` decision, so its signature must add `probe: &dyn TypeEnvProbe`. Callers, including
`parser/tests/ast.rs`, must pass `&NullProbe` accordingly.

**When `NameKind` is classified**: when `types.rs:162-165` constructs `Type::Name`, `state.probe()`
determines `kind`, fixing it once so that all later match arms no longer perform string comparisons.

**This eliminates all three entries in RFC-039 routing table C**—two already concerned only
parser-internal functions under the scope in 2.4, while the two genuine cross-layer references,
`declarations.rs:509` and `ast.rs:930`, are both covered here.

### 5.5 Merging `Expr::FnDef`

**Position: delete `Expr::FnDef` (`ast.rs:37-43`); function definitions uniformly use
`Expr::Lambda` + `StmtKind::Assign`.**

**This is the path already in use**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a parallel implementation that never executes.

**Change list** (12 consumption sites + 2 exhaustive arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                                  |
| ------------------------------------ | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, **delete this arm directly**)                                                  |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | As above; merge into the Lambda arm                                                                                                                     |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Format the Lambda in `Assign`; delete this arm                                                                                                          |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Obtain the name from `StmtKind::Assign { target, .. }` instead                                                                                          |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also carries `span`)                                                                                                |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                                  |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Check `Expr::Lambda` instead                                                                                                                            |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                         |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Check `Expr::Lambda` instead                                                                                                                            |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Check `Expr::Lambda` instead                                                                                                                            |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Check `Expr::Lambda` instead; the comment at `termination.rs:2303`, “`fn(): Never` — FnDef.return_type is the bare return type,” is updated accordingly |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                         |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                                    |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                        |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally identical to the first three fields of `FnDef`. The `name` field unique to `FnDef` has
no usable location at the `Expr` layer (the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: non-exhaustive `if let` matches such as `checker.rs:1642` and
`inference/expressions.rs:3374` become impossible to match after the change, with no error or
warning. Mitigation: the type-table gate (5.7) gains a “zero-construction variant” check. After
`Expr::FnDef` is deleted, writing `if let Expr::FnDef` again fails to compile because the variant no
longer exists. **This is a compile-time guarantee and requires no additional gate.**

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → delete.**

Rationale: its reason for existing (the comment at `ast.rs:244`) is “for the typechecker to use in
`classify_generic_params`”—a **dedicated slot reserved by the statement AST for the type checker**.
However, `extracted_params` in `declarations.rs:500-511` has already calculated `value_params` in
the same function, including type-position and constraint-position filtering, so carrying it in the
statement duplicates work.

Change: delete the field at `ast.rs:244-245` → delete the argument at `declarations.rs:472` → change
the data source for `classify_generic_params` to an in-place recomputation (it can obtain
`Lambda.params` from `value: Option<Box<Expr>>`). **Net result: one field declaration, one
construction site, and one transport path are removed.**

> **Risk**: `signature_params` is the **unfiltered** original list, while `Lambda.params` is
> **filtered** (the filter at `declarations.rs:500-511` removes type-position and
> constraint-position parameters). If `classify_generic_params` depends on the unfiltered version,
> behavior changes after the refactoring. **The actual list used by `classify_generic_params` must
> be verified in phase 1; equivalence must not be assumed.**

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but move the binder semantics out.**

The comments at `ast.rs:519-532` record its **sufficient reason for existence** (see 2.7): if `r` is
lost in `(r: P(m))`, the type checker can only guess that “a free variable not in scope in a
constraint is the return formal parameter,” and will **silently substitute the undeclared `m` as a
binder as well** (the comment marks this as a “measured defect”).

Therefore, it **cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**. `param` /
   `param_span` carry source facts only and do not bear type-layer binder semantics.
2. Move interpretation of the binder to `sema/`: when the type checker consumes `NamedParen`, it
   **explicitly** extracts `param` as the binder. This is “reading a syntax node's field,”
   equivalent in nature to `ConstExpr` embedding `Expr`—the syntax node provides facts, and the
   semantic layer interprets them.
3. Rewrite the comment from “the type checker can only guess” to “this node provides the binder
   name; semantic interpretation takes place in `sema/`.”

**Why this is a “migration” rather than a “deletion”**: the syntax tree must represent source
without loss. `r` genuinely exists in the source, so the AST cannot pretend it does not. The problem
is not the node's existence, but the failure to define responsibility for semantic interpretation
explicitly.

### 5.7 Generation-Time Type-Table Gates

**The reference mechanism has already been validated in this repository**: `build.rs:19-37` invokes
`parse_registry` / `validate` from `tools/code-tables`, performing uniqueness, segment, and
zh-completeness validation over 145 error codes; when `is_ok()` is false, it directly invokes
`panic!` to reject the build. `build.rs:39-54` then compares the RFC-013 markdown code table item by
item, likewise invoking `panic!` on a mismatch. **RFC-013 documentation and code therefore always
remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one `serde_json` dependency, and
the validation logic is text-level parsing without a dependency on syn.

The type tables currently have **no** similar gate. This document adds a mechanism in the same form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                               | Check                                                                                                                                                                                                                                                                                | Input                                    | Failure condition                                                                                                                                                                                                                                                                                                                                            | Remediation                                                                                                         |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site breakdown**                              | Scans the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, counts each variant's construction sites in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`) | Variant list + three-category hit tables | ① A variant has **zero forward construction** and is not registered; ② A registered variant has a forward construction site; ③ A registered wildcard-arm fallback exemption site does not match; ④ An **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered item by item and must not be generalized to every wildcard arm) | Delete the variant; or register a rationale / remove the stale registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2 Synonym table agrees with the repository-wide name lists** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `ast.rs:855` + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                             | Three tables                             | A type name exists in one table but is not recognized by another (compared according to each table's semantic mapping rules); **since WBS 6.1.2, T2.5, the “deleted-spelling sentinel,” also exists: adding back any of the 22 deleted spellings is a failure**                                                                                              | `type-tables --fix` synchronizes in both directions                                                                 |
| **T3 Built-in type names agree with RFC-011**                   | The canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                 | Two tables                               | Code-table range/name differs from the registry                                                                                                                                                                                                                                                                                                              | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                   |
| **T4 Exhaustiveness of `Type → MonoType`**                      | The `Type` variant list vs the match arms in `types/lower.rs`                                                                                                                                                                                                                        | Two tables                               | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                   | Add an arm / delete an arm                                                                                          |

**T4 implementation note**: Rust's exhaustive match already guarantees the “variant → arm” direction
at compile time. T4's value is in the **reverse** direction—detecting stale arms in `lower.rs` that
point to deleted variants. Such code fails to compile when the variant is deleted, so T4 is in fact
redundant. **T4 is therefore downgraded to a CI assertion** (`cargo build` succeeding is sufficient)
and does not enter `build.rs`'s `panic!` path. **T1–T3 are the three items that genuinely require
text parsing.**

**T1's two implementation difficulties and mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has only
   `serde_json`. Mitigations:
   - **Preferred**: introduce `syn` to `tools/type-tables` (only that crate; the root crate remains
     unchanged). `syn` is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the repository's existing line-level heuristic in
     `tools/extract_arm.py`—construction sites are on the right-hand side of `= ` or in
     function-argument position, while match arms are on the left-hand side of `=>`. Use the number
     of `=>` occurrences as an approximation.
   - **Conservative starting point**: when T1 first launches, **report only and do not fail**
     (`cargo:warning`), take one full baseline, and confirm that the heuristic is correct before
     changing it to `panic!`. This follows the existing handling of “CI gates are initially red and
     require a baseline first.”
2. **Reverse bridges and AST→AST reconstruction sites must be categorized separately**. Section 2.2
   records the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; 2.2.2 records the AST→AST reconstruction in the two substitution functions
   in `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites from the 13-variant
   list). If T1 combines them with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void`
   / `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` will be classified
   as “having construction sites” and exempted from deletion, directly contradicting the disposition
   table in 5.2. T1's report must separate the three construction-site categories, and **fallback
   exemptions must apply only to individually registered entries**—currently only the parser error
   fallback for `Void` at `types.rs:836`—rather than being generalized to all wildcard arms.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call in `build.rs`, immediately
after the existing block at `build.rs:19-37`, following the
`panic!("类型表校验失败（{} 项 error ...）", ...)` form.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                            |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                              |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                     |
| Monomorphization (`middle/passes/mono/`)                     | **Requires changes**. Eight locations in `mono/function.rs`—`358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623`—match `AstType::NamedStruct` / `AstType::AssocType`; if `AssocType` is deleted, all eight must change accordingly. **`function.rs:507-529`'s `mono_to_ast_type` must also change** (see #47 in 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Requires changes**. `ir_gen.rs:1266` and `1399` match `ast::Type::NamedStruct`; `ir_gen.rs:1380` has `let params: Vec<MonoType> = signature_params`, another consumer of `signature_params` that 5.6 must verify as well                                                                                                 |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Requires changes**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                        |
| Interpreter (`backends/interpreter/`)                        | **Requires changes**. `image.rs:44`'s `type_table: Vec<ir::Type` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, so the element-type change affects `{:?}` output formatting                                                                                                     |
| RFC-027 compile-time types                                   | **Requires changes**. `Type::ConstExpr` is retained but marked **compile-time only**. It disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                              |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by 07's first-layer validator through its “type consistency” invariant.
`ast::BinOp::Assign` / `ast::UnOp::Deref` (2.5) are the expression-side counterparts—they may appear
inside `ConstExpr`, and therefore likewise must not be lowered into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (the sole
   forward site for `Void` is an error fallback, registered as an exemption item by item) → the
   parser does not produce them. The disposition of reverse bridges and AST→AST reconstruction is
   covered by #28 / #47 in 6.3 and the 10 sites in 6.3.2.
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** in `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction of behavioral change, and it is a fix rather than a
   regression**, but it must be confirmed by 07's third-layer corpus differential.

**Semantic change points requiring close attention**:

| Location                | Current state                                                                                 | After convergence                          |
| ----------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retains the actual `MonoType::Generic`     |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retains the actual `MonoType`              |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Deleted** (an exhaustive match suffices) |

> **Design judgment**: the fact that these eight variants currently all collapse to `Void` means
> that **every path that depends on signature bytecode—the interpreter's arity/type checks and the
> serialization format—currently sees `Void`**. After convergence, these locations will see the
> actual types. **This is the only possible source of behavioral change, and its direction is
> “restoring correct information.”** If the corpus differential finds a difference, it should be
> treated as **exposing an existing defect**, not as a regression introduced by this document.
> Method: separately construct a `.yx` program containing a `Ref` parameter, and compare
> `dump_bytecode` before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                      | Change                                                                                                                                                                                                                                           | Category                 |
| --- | ---------------------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`    | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                                          | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                 | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                           | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                   | Delete `Expr::FnDef`                                                                                                                                                                                                                             | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                 | Delete `Assign.signature_params`                                                                                                                                                                                                                 | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                    | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                              | Follows #3               |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                 | Change the `NamedParen` comment to “syntax provides the binder name; semantic interpretation takes place in `sema/`”                                                                                                                             | Documentation            |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                           | Add `pub enum NameKind`                                                                                                                                                                                                                          | New                      |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                       | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                               | New                      |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                   | Add the `probe` field and accessor                                                                                                                                                                                                               | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                 | Use `state.probe()` to determine `NameKind`                                                                                                                                                                                                      | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                 | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; only add a note about the variant deletion)                                                                                                                | Documentation            |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                     | Delete the `_ => (Vec::new(), Type::Void)` fallback; replace it with an explicit error return                                                                                                                                                    | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                 | Delete `n == "Terminates"`; change it to `state.probe().is_predicate(n)`                                                                                                                                                                         | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                 | Delete the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                         | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                     | Delete the `signature_params` argument                                                                                                                                                                                                           | Follows #4               |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                      | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                     | Follows #3               |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                     | Retain (the parameter-list retrieval point; changed with 5.5 to read Lambda directly)                                                                                                                                                            | Follows #3               |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`    | Delete 13 match arms (one for each deleted variant)                                                                                                                                                                                              | Follows #1               |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                 | Cut the synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                               | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                     | Retain the `DateTime` alias                                                                                                                                                                                                                      | Retain                   |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                       | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                     | New                      |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                             | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                          | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`) | Delete `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                            | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                     | Keep `FunctionCode.params` as `Vec<MonoType>` (already so)                                                                                                                                                                                       | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                              | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                            | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                     | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                     | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                    | Change the `type_table` conversion to `.collect()` (the elements now have the same type)—the actual change is to **delete the conversion and transport the values unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                             | Follows #26              |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)        | **Delete** the entire `From<MonoType> for IrType` impl (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                    | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                      | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                    | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)                 | Handle the `AstType::AssocType` arms according to the gate's conclusion                                                                                                                                                                          | Pending                  |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`              | Type-table gates T1–T3                                                                                                                                                                                                                           | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                     | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                     | Follows #3               |
| 33  | `spawn/analysis.rs`                                                    | `912`                                     | As above                                                                                                                                                                                                                                         | Follows #3               |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                      | As above                                                                                                                                                                                                                                         | Follows #3               |
| 35  | `orchestrator.rs`                                                      | `1492`                                    | As above; obtain the name from `Assign.target`                                                                                                                                                                                                   | Follows #3               |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                            | As above                                                                                                                                                                                                                                         | Follows #3               |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                    | As above                                                                                                                                                                                                                                         | Follows #3               |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                    | As above                                                                                                                                                                                                                                         | Follows #3               |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                    | As above                                                                                                                                                                                                                                         | Follows #3               |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                      | As above                                                                                                                                                                                                                                         | Follows #3               |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                     | As above                                                                                                                                                                                                                                         | Follows #3               |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                    | As above                                                                                                                                                                                                                                         | Follows #3               |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                             | As above + comment update                                                                                                                                                                                                                        | Follows #3               |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                 | Delete the `Expr::FnDef` test                                                                                                                                                                                                                    | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                 | Rewrite as Lambda + Assign                                                                                                                                                                                                                       | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`           | Delete the lowering assertions for the 11 dead variants                                                                                                                                                                                          | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                 | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type`; uniformly use `AstType::Name` + `mono.type_name()`. **Must be handled in the same batch as #1**                                        | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                              | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)` instead. Callers, including `parser/tests/ast.rs`, pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                   | Delete the local `name_used_as_type`; call the merged implementation in `ast.rs` instead; change the `is_predicate` short circuit to read `NameKind::Predicate`                                                                                  | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                     | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3)                                                                                                                                                                   | Convergence              |

> **The six completed lines in 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed
> 2026-10-28) implemented all six in one batch—four carriers switched to `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType`
> and the `ir::Type` re-export alias were deleted, and decoding now transports values unchanged
> (`bytecode.rs:2315`, `:2316`, `:2346`). **These lines' line numbers have been retaken against the
> implemented state** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390` were
> pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured results and gate
> outcomes.

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, needs a `mod`
> declaration wired in (this repository has already learned the lesson of 8 orphan test trees).
> Register the new crate in the root `Cargo.toml` when creating it.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the 293 `.yx` corpus files under `tests/yaoxiang/` (single-file) + the multi-file
corpus layer established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard
library **must all remain behaviorally unchanged** (same diagnostic codes and messages, with
ordering normalized as needed).

| Risk                                                                   | Assessment                                                                                                                                                                                                                                                                                                        | Mitigation                                                                                                                                                                                                          |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code uses type annotations such as `i64` / `int` / `f64`        | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                 | **Measured** (end of 5.3): 0 noncanonical names in actual type positions → 6.1.2 directly deletes the 22 spellings, with **no lexer alias table**; the corpus diagnostic set is unchanged                           |
| Source code uses `DateTime`                                            | Retain only the capitalized temporary compatibility alias (`mono.rs:883`); it is not a built-in name                                                                                                                                                                                                              | Moving it out of the built-in name table belongs to P10 S11 (issue #513); until then, `DateTime` and `Int` are synonymous, with no behavior change                                                                  |
| `("()")` occurs in a type position                                     | It is not a product of the `Name` form, but a spelling of the `Void` type (standard-library signature strings write `() -> Void`)                                                                                                                                                                                 | **Retain**: `()` shares the `Void` arm and is one of the finalized 15 names                                                                                                                                         |
| `dump_bytecode` output changes                                         | The `type_table` element type changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                          | **Permitted by the C3 criterion** (07 specifies that `dump_bytecode` is compared only during C1/C2). However, the output diff must be recorded in phase 2 for manual review                                         |
| Behavior changes after the lossy bridge is fixed                       | See 6.2                                                                                                                                                                                                                                                                                                           | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                           |
| Type-substitution results change after `mono_to_ast_type` is rewritten | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After the variants are deleted, the `_` arm may change the result from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Targeted comparison in phase 3: select a corpus with generic substitution and compare the `type_name()` of the monomorphized output before and after the change. **Do not assume that `{:?}` output is equivalent** |
| `.yx` source code itself                                               | **Zero modifications**                                                                                                                                                                                                                                                                                            | This document does not require changes to any corpus or std source file. This is a hard acceptance item                                                                                                             |

**Hard acceptance condition** (**scope revised on 2026-10-27, following the 2026-10-26 decision
②**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly driven
test files** under `tests/` may be modified when **mechanically required**, with each change
recorded by **file / line count / rationale**, and **tests must not be deleted or assertions
weakened**. The original scope (`git diff --stat tests/ src/std/`) imposed a zero-diff requirement
on all of `tests/`; this revision relaxes the scope by one level for directly driven test files
only, while preserving the zero-diff requirements for the corpus layer (`tests/yaoxiang`), baseline
layer (`tests/baselines`), and standard library (`src/std/`). **Exception**: the 6.8.2 behavior-fix
batch is handled as a separate round with a separate decision.

## Implementation Essentials

The criterion uniformly uses 07's **C3: same diagnostic codes and messages (ordering may be
normalized)**. The following is the phase breakdown for this document's workstream; the global
cross-document ordering is covered by RFC-039.

| Phase                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Rollback point                         |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3. The first version of T1 **reports without failing**. Take a full baseline against the **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                           | The T1 report accurately lists the 13 variants with **zero parser-forward construction** (including the classification of `AssocType`: forward 0 / AST→AST 2, D8 unchanged), and **separately lists the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; the 13 zero-forward-construction variants are reported separately from the 1 globally zero-construction variant (`Enum` only); T2/T3 are green or provide a repair list | No code changes; no rollback required  |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) converged to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; the 7 lowercase copies in `add_builtin_types`, the `Uint` / `Int128` entries in the LSP list, and the `Uint` arm in `sizeof` were cleaned up in the same batch; **no lexer alias table** was established (0 noncanonical names measured in the corpus). **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`**                                                                         | C3: the corpus-wide diagnostic set is **identical item by item** to the phase 0 baseline                                                                                                                                                                                                                                                                                                                                                                                        | Separate commit; revertible            |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers switched to `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy bridge `From<MonoType> for IrType` (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias (`ir.rs:3`'s `pub use`, reduced to a private `use`) were deleted; decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), so the prior silent information loss in which eight `MonoType` variants collapsed to `Void` no longer occurs | C3 + targeted `dump_bytecode` comparison (each row in the 6.2 table)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                   | Separate commit; revertible            |
| **3. Dead-variant deletion**                  | Delete the 13 variants in the 5.2 table; remove match arms such as `mono.rs:699-708` and the `types.rs:836` fallback; **handle `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; update `types/tests/mono.rs` in parallel                                                                                                                                                                                                                                                                                                                                                              | C3 + a fully green `cargo build` (exhaustive matches force completion of all arms) + a targeted generic-substitution comparison (last row of 6.4)                                                                                                                                                                                                                                                                                                                               | Separate commit; revertible            |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); classify `NameKind` at `types.rs:162-165`; merge the two `name_used_as_type*` implementations (#49)                                                                                                                                                                                                                                                                                                                                                                                                           | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                              | Separate commit; revertible            |
| **5. Dead AST variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; migrate `NamedParen` semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | C3 + all 18 modules in `tests/integration/` green                                                                                                                                                                                                                                                                                                                                                                                                                               | Two commits (FnDef / signature_params) |
| **6. Gates become hard**                      | Change T1 from warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                        | Separate commit                        |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow phase 2**—before phase
2 lands, the bytecode side's reverse bridge A (`bytecode.rs`'s `From<MonoType> for IrType`) consumes
`IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation. That
bridge was deleted in 6.2.2; **the remaining rewrite surface when phase 3 deletes the variants is
reverse bridge B** (`middle/passes/mono/function.rs`'s `mono_to_ast_type`, which still
reverse-resolves `String` / `Bytes` by name) and the three `ast::Type` carriers in
`middle/core/ir.rs`. Phase 3's #47 must be handled in the same batch as #1; otherwise,
`mono_to_ast_type` silently changes behavior.

**Downstream dependency**: only after all phases in this document are complete may
[04-ssa.md](04-ssa.md) begin. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all have the same element type, giving SSA type annotations a single destination.

### Mandatory Verification for Each Phase

| Phase | Required execution                                                                                                                                                            |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the separately listed reverse-bridge reconstruction sites**) |
| 1–5   | `cargo test`; full-corpus differential over `tests/yaoxiang/` (07's third layer); item-by-item comparison against the C3 criterion                                            |
| 2     | Additionally: construct a `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                              |
| 3     | Additionally: select a corpus with generic substitution and compare the `type_name()` of the monomorphized output before and after the change                                 |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                                        |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                               |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                       | Rationale                                                                                                                                                                                                                                      |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias        | The alias at `ir.rs:3` directly causes the mismatch between the `BytecodeFunction.params` and `FunctionCode.params` field types (2.1). Once the alias is deleted, both field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                      | It would introduce an L2→L3 reverse dependency, violating RFC-039's layering, and would require adding `Span` / `ConstExpr` back to `MonoType`. See the complete argument in 5.1                                                               |
| Do not delete `NamedParen`   | Retain it as a pure syntax node; move binder semantics to `sema/`                                                | `ast.rs:519-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as a binder                                                                             |
| Directory rename             | **Do it** (D1), as a pure-move batch at the end of P5/P6; this document's body uses the current pre-rename paths | 2026-10-03 decision (ChenXu233); 2026-10-04 supplement on implementation method                                                                                                                                                                |

### Rejected Alternatives

None of the four alternatives was adopted. **Retaining all three representations and adding only a
conversion layer** does not address any root cause; `ir::Type` remains a non-independent type, and
none of the dead variants, synonym table, or reverse dependencies disappear. **Generating the
synonym table with a macro** treats a symptom rather than the cause—after `Type::Int(usize)` is
deleted, the macro has no reason to exist, and T2 provides consistency guarantees that are more
verifiable than macro expansion correctness. **Deleting only dead variants without unification** has
the lowest cost but leaves the problem in place, with no gate preventing the next person from adding
another `Type::Int128(usize)` and recreating the same dead variant; if only one commit is possible,
it must explicitly be identified as a subset of this alternative and deliver the T1 gate as well.
**Unifying on `MonoType`** is covered in 5.1.

### Benefits

- **Eliminates maintenance burden at 14 locations**. The 12 consumption sites + 2 exhaustive arms
  for `Expr::FnDef` form a parallel path that never executes; deleting it removes one branch each
  from `ir_gen.rs`, `ownership.rs`, and `termination.rs`.
- **Eliminates one duplicated implementation that has already diverged**. The two copies of
  `name_used_as_type` (2.4) differ in recursive field traversal; after merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value”**. This is a
  prerequisite for 07's first-layer “type consistency” invariant to pass truly.
- **The gates make regression impossible**. T1 turns “adding a zero-construction variant” from a
  silent pass into a build failure—the existing `build.rs` mechanism proves that this approach
  works.
- **Zero source-code changes**. The 293 `.yx` corpus files and the std library require no
  modifications, which is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may cause behavioral changes**. After the lossy bridge is fixed, the eight `MonoType`
  variants expose their actual types rather than `Void`. The direction is a fix, but it changes
  paths that depend on signature bytecode. This is the largest uncertainty in this document.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify. This is why phase 0 requires manual
  verification.
- **If T1 misses reverse bridges, the disposition table and gate will contradict each other**.
  Section 2.2 proves that seven “dead variants” have production reconstruction sites in reverse
  bridges. If T1 does not separate construction-site directions, those seven will be classified as
  “having construction sites” and exempted from deletion—the gate will then obstruct the deletion
  work instead of safeguarding it. | **2. Deleting `signature_params` carries semantic risk**. It
  carries the **unfiltered** parameter list, while `Lambda.params` is **filtered**. If
  `classify_generic_params` depends on the former, the refactoring changes generic-classification
  results. **This must be verified first; equivalence must not be assumed** (the `signature_params`
  consumer at `ir_gen.rs:1380` must be verified as well). | **3. Phase 3 touches
  `middle/passes/mono/function.rs`**. Besides the 8 `AssocType` matches, `mono_to_ast_type` at
  `507-529` is the exit point for monomorphized type substitution, and its rewrite may change the
  substitution result's `type_name()` (last row of 6.4).
- **The `.yx` corpus may contain a non-zero number of noncanonical type names**. If the corpus
  extensively uses `i64` / `int`, phase 1's direct deletion of synonyms is infeasible and must be
  replaced by moving them into a lexer alias table, reducing the scope of `from_builtin_name`'s
  convergence. (**Measured as 0**: the 6.0 survey is at the end of 5.3, so WBS 6.1.2 directly
  deleted the aliases and did not take the alias-table route.)
- **The gate will initially be red**. If T1 invokes `panic!` immediately on first launch, it will
  block compilation. It must first report only and establish a baseline.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document retains no pending items.**

## See Also

### Parent and Peer Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — upper-level master plan; four-layer model, criterion tiers, routing table C (3
  type-representation entries assigned to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction specification,
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; convergence of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, first-layer IR validator's type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  `Type::Generic` unified path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — built-in type table
  (comparison target for gate T3)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of a generation-time gate in `build.rs`
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — definition of the 26 `ast::Type` variants
- `src/frontend/core/parser/ast.rs:519-540` — binder semantics of `Type::NamedParen` (evidence that
  deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:241-249` — typechecker-specific
  `StmtKind::Assign.signature_params` field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: a duplicate of
  `declarations.rs:42` that has already diverged
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its
  boundary-crossing call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — parser represents every primitive type as
  `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — the sole forward construction site of
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — expression positions are represented as an
  empty-body `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm retrieves the parameter list
- `src/frontend/core/types/mono.rs:839-886` — `from_builtin_name` built-in name resolution table (15
  names + the temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (approximately 22 variants, starting at
  `mono.rs:167`; 1,077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation (1,128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — evidence that `ir::Type` is an alias of `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction`'s `Vec<ir::Type>` fields
- `src/middle/core/bytecode.rs:2352-2390` — lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `FunctionCode`'s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge reconstructing
  the seven “dead variants”
- `src/backends/interpreter/image.rs:44` — interpreter module's `type_table`
- `build.rs:19-37` — error-code registry generation-time gate (form for the type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
