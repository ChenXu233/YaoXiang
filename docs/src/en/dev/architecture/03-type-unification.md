# Type Representation Unification

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, acceptance-criteria tiers, and execution-stage
> ordering; see the [index of this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

### Coverage

Consolidate the multiple parallel type representations currently in use into one, thereby
eliminating the four categories of resulting burden:

- **13 `ast::Type` variants with zero parser-forward construction sites** (among the 26 variants in
  `ast.rs:427-541`, see 2.2; the “forward” metric does not subtract reverse bridges or AST→AST
  reconstruction sites)
- **A hand-written table of type-name synonyms** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser's genuine reverse dependency on the typecheck layer** (2 sites, see 2.4)
- **One expression variant with zero production construction sites**, `Expr::FnDef` (**removed: WBS
  6.5.1, 2026-10-30**—the variant definition and all consumer arms were removed, see 2.6)

The concrete deliverables are four items: a disposition table covering all 26 variants individually
(5.2), the semantic classification `NameKind` and elimination of the synonym table for `Type::Name`
(5.3), an injectable `TypeEnvProbe` data flow (5.4), and build-time type-table gates T1/T2 (5.7; the
original T3 was removed following the user ruling of 2026-10-30).

### Directory Renaming and Paths Used in This Document

RFC-039 decision D1 mandates directory renaming (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`),
to be **completed together with P5/P6, without splitting it into stages**. This document's 50-item
change list and all acceptance greps are written against the **pre-rename paths**—they constitute
the implementation baseline for the P6 batches **before the rename batch**; the directory rename at
the end of each batch is a **pure-move batch** (C1 zero-diff) committed separately. The baseline
becoming invalid after the rename commit is expected; use git history as the bisection reference.

### Out of Scope

- **How the directory rename is implemented**. `Type` is not moved out of `parser/ast.rs` in this
  phase; only a new `lower.rs` is added under `types/` as the single conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 under RFC-039 decision D1, in
  the form shown in the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Grading and validator implementation for equivalence criteria**. The definitions of C1–C6 and
  the three tiers of criteria (IR validator / normalized snapshots / corpus differential) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document cites only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reasons
  given below.
- **The global P1–P10 execution sequence**. See the main body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document provides only the stage breakdown for
  its own workstream (see “Implementation Notes”).

### Division of Responsibility with RFC-039

RFC-039 is the overarching blueprint: the four-layer model, criterion tiers, cross-document stage
ordering, and the rationale for undertaking this refactoring now. This document is the
implementation blueprint for **type-representation convergence**, one of the items in RFC-039:
exactly which variants to delete, which files to change, in what order, and which gates prevent
regression.

All line numbers, variant lists, and construction-site counts in this document come from static
verification of `src/`; each item cites its evidence as `file:line`. **This document does not record
the execution process; it records code facts and the resulting dispositions.**

To facilitate cross-references with documents from the same batch, the “Target Design” and “Detailed
Design” chapters retain the `5.x` / `6.x` subsection numbering, while the remaining chapters follow
this directory's standard style.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(`ir.rs:3`). [04-ssa.md](04-ssa.md) is to introduce register type annotations and new instructions
into the IR. Without first consolidating the representation, SSA type annotations would grow
directly from the `ir::Type` alias onto the 26-variant syntactic type, effectively
institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree inside a type.
  An `Expr` cannot appear in an IR instruction's type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently cement the 13 dead variants
and the two syntax-specific variants into the IR contract.

## Current State

Everything below is a statement of code fact and contains no proposals. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 has landed)**: This section records the state **before** WBS
> 6.2 landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) has changed all four type carriers to carry
> `MonoType`, removed the lossy bridge `From<MonoType> for IrType`, and removed the `ir::Type`
> re-export alias; all statements below concerning `ir::Type` or the lossy bridge are therefore
> **pre-change facts**, while items marked “(deleted in 6.2.2)” record dispositions that are now in
> effect. See [09a](./09a-wbs-history.md) §P6 6.2 for implementation details and measured figures.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                              | Causal relationship to type representation                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                 | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, the parser chooses the latter, and typecheck must supply a table to translate the former back                                                                         |
| Bytecode-layer signatures differ from IR-layer signatures                       | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias, so `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places, with no checks | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; both locations duplicate the mapping and do not validate each other                                                                                                                |
| `"Terminates"` is hard-coded in the parser                                      | Because the parser cannot determine whether something “is a type” or “is an operator interface,” it can only embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                           |

**The first three items are representation problems; the fourth is a downstream side effect of a
representation problem.** Doing only item 1 (deleting dead variants) would leave the synonym table
and reverse dependency in place; doing only item 4 (injecting a callback into the parser) would
build on top of an still-inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition location                       | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, and `ir.rs:6` then executes
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two “type” symbols
simultaneously, even though one is merely an alias.

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was deleted in 6.2.2).** `From<MonoType> for IrType`
(previously at `src/middle/core/bytecode.rs:2353-2390`) downgraded `MonoType` to `ast::Type`, and
its **information loss was silent**:

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

All 8 `MonoType` variants collapsed into `IrType::Void`, in addition to an `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; the accompanying comment admits that this is a “transitional
branch before deleting the enum variant”).

Here, `IrType::Void` is a **legal type value rather than an error signal**. Downstream code
therefore cannot distinguish “this function really returns `Void`” from “this function's return type
was lost during bridging”—one reason why the “type consistency” invariant of the first-layer
validator in 07 cannot currently pass.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants targeted in 2.2—**both ends
of the lossy bridge point to dead code**.

**Current state (from 6.2.2)**: The impl has been **removed in its entirety**, and decoding no
longer downgrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now transports the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those 8
`MonoType` variants retain their true types in signatures and type tables, and `IrType::Void` is no
longer used as a substitute for “information loss.” The `ir::Type` re-export alias
(`pub use ... ast::Type` at `src/middle/core/ir.rs:3`) has also been downgraded to a private import
(`:7`)—the IR layer no longer publicly re-exports the syntax-layer type; however, the IR layer still
has three carriers that directly hold `ast::Type` (`Instruction::Cast.target_type` /
`Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`), deferred to 6.3.

### 2.2 Parser-Forward Variants with Zero Construction Sites

**Measurement definition**: The construction-site scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. **A construction site is an expression-position `Type::<variant>` path** (pattern
positions do not count), and results must be listed separately in **three categories and not
conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridges** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Among the 26 variants of `ast::Type`, the following 13 have **zero construction sites in the
parser-forward direction** (reconstruction sites in the latter two categories are described in 2.2.1
/ 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                        |
| ------------------------------------ | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                      |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                      |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                      |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                      |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                      |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                      |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback**) `_ => (Vec::new(), Type::Void)`, registered as an itemized exemption; eliminated with 6.4 #12                                                                                                                                                |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                      |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                           |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                           |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                           |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                           |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 unchanged** |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: The “Definition” column in
> this table and the 5.2 table was written on 2026-10-05. Seventeen lines were subsequently inserted
> before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so the definition lines of
> **all 26 variants shift uniformly by +17** (for example, `Int` moves from 432 → 449 and
> `NamedParen` from 533-540 → 550-557). This round only **registers** the drift and does not modify
> the table body; subsequent references use the `ast::Type` symbol anchor.

The causal chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… they follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site of `Type::Void` is an error
   fallback**—`_ => (Vec::new(), Type::Void)` at `types.rs:836`, located at the end of a match
   returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`; they are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document says “**zero parser-forward construction**”: this is neither “zero
references globally” nor “zero production construction”—11 of these items do have production
construction sites in the other two categories (reverse bridge / AST→AST reconstruction). These
tests themselves codify the behavior under test: they assert precisely what these syntactic forms
lower to as `MonoType`, even though the parser never produces those forms. When the variants are
deleted, these tests must also be deleted or rewritten.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The table's “zero forward construction” status holds only in the **parser → AST forward-construction
direction**. In the reverse direction, there are two production `MonoType → ast::Type`
reconstructions that **actually construct 7 of the variants listed above**:

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
    // #299: Container types such as String/Bytes are now Generic and are converted back to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This scope boundary directly affects the disposition**:

- The `bytecode.rs` site disappears with 6.3's #28 (deleting the entire `From<MonoType> for IrType`
  impl), so it creates no additional cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name reverse resolver (annotated `#299`), converting `String` / `Bytes` from `Generic` back
  into dedicated variants by name. After deleting `ast::Type::String` / `Bytes`, these two guard
  branches must be rewritten; after deleting `Int` / `Float` / `Bool` / `Char` / `Void`, the
  behavior of the `_` arm changes (some types that previously fell into `_` now silently take the
  `AstType::Name` path). This is item 47 added to 6.3.
- **Direct consequence for the gates**: A T1 gate that scans only forward construction sites would
  classify those 7 variants as “having construction sites” and exclude them from the removable list.
  T1 must explicitly include reverse bridges in its measurement scope; otherwise, its conclusions
  conflict with the disposition table in this document (see 5.7).

#### 2.2.2 Scope Boundary (2): AST→AST Reconstruction (the third category added in this revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution (reconstructing an
existing AST type, **not** `MonoType → AST`). The P6 T1 gate measurement (6.0.2 baseline,
2026-10-27) found **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to the
13-item list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these 5 variants, it must rewrite these 10 sites in the same batch** (09's 6.3.2
has registered them); otherwise, the corresponding arms will no longer compile. **Implemented (WBS
6.3.2, 2026-10-29)**: all 10 sites were removed, and the disposition table is in
[09a](./09a-wbs-history.md) §P6 6.3.

### 2.3 Hand-Written Synonym Table

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

**The sole reason this table exists is to bridge the inconsistency in the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent at the AST layer but
structurally different (and the parser produces only the latter), `from_builtin_name` must accept
case variants, abbreviations, signed names, and the `DateTime` alias. The appearance of `"()"` on
the `Void` line is especially revealing—it works with the `Type::Void` fallback at `types.rs:836`.

The same knowledge of “which names are built-in type names” exists in seven copies across the
repository, with no consistency validation whatsoever:

| #   | Location (**measured on 2026-10-27**; this section was written on 2026-10-05, and the original line numbers have drifted)            | Form                                                                                                                                                                                                                                |
| --- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured at 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                                     |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                              | String match, copying the three names `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                            |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured at 855-858)                                                         | A `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                          |
| 4   | `src/lsp/world.rs:182`                                                                                                               | The LSP-side list of built-in type names (14 names = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                            |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured at 407)                                           | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` family of aliases (registered by the P0 gate on 2026-10-06)                                               |
| 6   | The base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured at 497-512)                       | String match, name → byte size. This previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **A dead arm and live evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured at 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set is a name” rule (`inference/expressions.rs:2917-2918`, decision 4 of #300)                                                      |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: The line numbers of all seven
> copies in this section **have drifted** (item 1 by +223 and the others by between 2 and 9 lines),
> but **the number and content of the entries have not changed** (item 1 had 13 arms when this
> section was written, item 3 has 15 entries, and item 4 had 15 entries when this section was
> written; after WBS 6.1.2, item 1 has 15 arms / 16 spellings and item 4 has 14 entries). Subsequent
> references use the **symbol anchor + repository path**; the line numbers in this table are for
> location only. Item 1's arm count and the live drift evidence in items 5 / 6 / 7 (the dead `Uint`
> arm and the `Set` contradiction with typecheck) **all remain**, and this section does not modify
> code.

`CONST_PARAM_TYPES`, item 3, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured at 932) to determine const generic parameters. It
enumerates “type names usable as const parameters” and therefore **partially overlaps but is not
identical to** the full set in `from_builtin_name`, with no mechanism ensuring that the two remain
synchronized. Item 4 is a fourth copy used for LSP syntax highlighting and hovers, likewise without
validation; T2's comparison scope includes it.

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 / 7 share the same root as items 1–4
(string-keyed tables from built-in names to semantics / IDs), though their purposes differ and their
drift risks are the same. The same gate also found 11 string dispatches for built-in container names
on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4,
etc.)—these are **outside both this section and the parser data-flow scope in 5.4**; the inventory
and disposition are registered in [06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 under the
P6 extended-verification items.

### 2.4 The Parser's Reverse Dependency on the Type Layer

Routing table C in RFC-039 records 3 entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both of which belong to the type layer).
**Measurement differs from that description**: these two functions are defined inside the parser
itself, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository's only `fn`
  definition)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the repository's only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production code sites** that genuinely reference `typecheck` across layers,
and both call the same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                 | Purpose                                                                                               |
| --- | --------------------------------------------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Determines constraint-position parameters (`T: Add`), which do not occupy runtime parameter positions |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Determines constraint-position parameters and produces a `GenericParamName` with `constraints`        |

The second site is more deeply embedded—**`parser/ast.rs` itself** (the entire `ast.rs:847-948`
block shares a file with `StmtKind` / `Expr`) directly references a typecheck symbol. Routing table
C records only the `declarations.rs` site.

Together with the hard-coding at `declarations.rs:498-499`, it forms one combined decision:

```rust
// declarations.rs:498-499
// Expanded as more built-in predicates are added (the future open-set approach: pass predicate names down to
// the parser, or change them to a form decidable by the syntax layer).
// The parser does not hold a type environment, so it checks two sources: hard-coded built-in predicates,
// plus declarations accumulated during the current pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment admits that this is the temporary form of a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask whether a
name “is an operator interface name” because it must distinguish whether `Type::Name` refers to a
type or to an interface constraint. **`Type::Name { name: String }` uses an undifferentiated string
to carry at least 4 meanings** (concrete type / type variable / constraint name / predicate name),
and the syntax layer cannot determine this itself, so the authority is forced upward into the type
layer.

#### Duplicate Implementation: Same Name and Meaning, but Behavior Has Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` acknowledges
this:

```rust
/// Whether a parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; implemented independently here to avoid
/// a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations have **already diverged**:

| Dimension               | `name_used_as_type` at `declarations.rs:42-80`                          | `name_used_as_type_in` at `ast.rs:847-885`           |
| ----------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                 | No callback                                          |
| `Type::Generic` branch  | First short-circuits to `false` via `is_predicate(app_name)` (`:54-56`) | No short circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                    | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                              | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records the origin of the latter two branches: “Previously, these
two branches were missing, so a const generic parameter referenced in a definition body could not be
detected as ‘used as a type,’ and annotation validation incorrectly reported a valid const parameter
as an unknown name.”

In other words, **the copy made to avoid a cross-module dependency did not receive subsequent
fixes**. The `ast.rs` version recursively traverses fields in a definition body, while the
`declarations.rs` version does not—the same source fragment produces different results at the two
decision sites. This duplicate must be merged when `NameKind` is introduced in 5.3.

### 2.5 Parallel Operator Enums

There are two operator enums, which do not reference each other and **whose members have already
diverged**:

| Concept         | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                  |
| --------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operator | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operator  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Each also independently implements a complete `Display` (`const_data.rs:291-317`) and classification
predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at
`const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in type position, they fall into
`Type::ConstExpr` (`ast.rs:506`), namely the “compile-time-only variant.” This is directly related
to the IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears following
`Type → MonoType`, and the IR can never see it, while `Assign` / `Deref` continue to exist through
paths such as `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
Because the two enums have diverged, neither side can independently determine “which operators may
occur in type position.”

This phase does not merge the two enums (their ownership is discussed under “operator-change
surface” in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate's scan targets
should include `ast::BinOp` / `ast::UnOp`, so that newly added zero-construction operators are
equally visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction Sites (Removed: WBS 6.5.1, 2026-10-30)

**This section records an already-landed fact**: `Expr::FnDef` was once a variant of `ast::Expr`.
The claim of **zero production construction** is correct—across the entire repository, there were
only two construction sites (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`), both in tests.

However, **there were far more than 4 consumption sites**: the baseline measured **15 production
consumption sites** (classified in 5.5), plus 6 comments mentioning the variant; in the 6.4 closure
state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1 definition + 15 production
consumption sites + 3 test construction sites + 6 comments.

**The actual path taken by a function definition** is: `pratt/nud.rs` constructs `Expr::Lambda`, and
`parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in `value`).
In other words, the `FnDef` branch was **a never-executed parallel path maintained by 19 code
locations** (15 consumption sites + 3 test construction sites + 1 definition).

**Landed state**: The variant definition and all consumer arms were **removed completely**;
`git grep -n "Expr::FnDef" -- src/` within `src/` has 0 matches (only 2 explanatory comments
containing the bare word `FnDef` remain). See [09a](./09a-wbs-history.md) §P6 6.5 for the per-site
disposition table, test migration, and measured figures; see 5.5 for the design of the merge.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` formerly carried typechecker-specific fields (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>` (formerly
`ast.rs:244-245`), and the comment explicitly stated that the “first parameter group of the
signature is preserved verbatim (including parameter names) for typechecker
`classify_generic_params` use”—the statement AST reserved a dedicated slot for the type checker.

The current representation places parameter names and types **together** in the type annotation's
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` instead carries the declared
signature `type_annotation: Option<Type>` (the same carrier as `Assign`).

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and treats it as a parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module's match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:518-532` explicitly states:

> RFC-027 §3's return-position refinement relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable outside the scope of the constraint is the return-value formal parameter”; consequently,
> `(r: P(m))` silently substitutes the undeclared `m` as a binder as well (observed defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is downgraded to the type checker's internal working representation; the `ir::Type` alias
is deleted.**

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

**Why not “merge into `MonoType`”** (the most readily proposed alternative, rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), while
   `MonoType` does not. C3 in 07 requires “the same diagnostic codes and messages”—diagnostic
   positions must remain traceable to source code. Adding `Span` to `MonoType` would burden the
   monomorphized type with a collection of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker's solving
   process**, not types themselves. Putting them in the sole type structure would require the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive reverse argument**: making `MonoType` the sole representation would make the L2
   syntax layer depend on the L3 semantic layer's type. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this document's objectives, so
   choosing a design that creates a reverse dependency would be self-contradictory.

**Why `ir::Type` must be deleted rather than “retained”**:

The alias at `ir.rs:3` forces `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType`
(`codegen/bytecode.rs:277-278`). The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed in its
entirety, thereby also eliminating the reverse reconstruction recorded in 2.2.

**Explicitly excluded**: `Type` is not moved from `parser/ast.rs` to `sema/types/` in this phase.
This phase only adds `lower.rs` under `types/` as the sole conversion point and leaves the
definition location of `Type` unchanged; the directory rename is performed under D1 at the end of P6
as an independent pure-move batch—at that point, `Type`'s home will be the top-level AST domain at
`ast/type_.rs`, while `sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> **Landed (WBS 6.3, 2026-10-29)**: All 13 “delete” items in this table have **been
> implemented**—`ast::Type` has converged from 26 variants to 13 (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were **rewritten in the same batch**. See [09a](./09a-wbs-history.md) §P6 6.3 for
> measured figures, the 10-site disposition table, and criteria.

> Disposition categories: **delete** (zero parser-forward construction), **retain** (has forward
> construction sites), **migrate** (semantics move to a different node).

> **Line-number convention**: The “Definition line” column contains the variant's **original
> pre-deletion position** (the 26-variant era; 13 were deleted in 6.3); the match-site line numbers
> in the “Rationale” column use the same **pre-6.3 baseline** and do not shift as code moves—this
> batch only resets the definition lines of **variants that still exist** (rows 14 and 26) to their
> landed positions and corrects an obsolete consumption site in row 14.

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthen)                    | The parser's **sole** output path for primitive types (`types.rs:162-165`). After deleting the 12 primitive-type variants, all primitive types use it. A “semantic kind of name” classification must be added (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:699` is a match arm). Bit width is carried through the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:702` is a match arm). **Note**: `IrType::String` at `bytecode.rs:2372` consumes it, and that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:703-706` are match arms). `bytecode.rs:2373` likewise consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The only parser-forward construction site is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an itemized exemption). The fallback is eliminated together with 6.4's error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has a production construction site at `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:747` is a match arm); zero across all three categories (the only globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; this is unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                                 | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering) (the former `bytecode.rs:2366` referred to `IrType::Fn` and disappeared with 6.2's `ir::Type` alias and lossy bridge); **field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types together) is part of 6.8.1 (D54)**—`Assign.signature_params` names are inherited from there (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous`/`from_type`/`is_named`; anonymous parameters (pure type positions) are synthesized with empty names, so this table's “net effect” is unchanged; `Param` has a hand-written `Debug` implementation (anonymous parameters print using their carried type ⇒ zero IR snapshot drift, see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) rewritten in the same batch as 6.3.2. `types.rs:392-396` already states that it uses the `Generic` path                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero parser-forward construction (the syntax layer has no `::` associated-type production path); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 unchanged**: measurement follows the forward direction; if associated-type syntax is enabled in the future, a new proposal at that time will define it anew                                                                                                                                                                                                                                                                                                                                                  |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has a production construction site at `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw-pointer type in unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites at `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has a production construction site at `types.rs:160`. It is the only `Type` variant embedding an `Expr`, and the IR layer is forbidden from carrying it (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying's terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | The semantics (binder name) move to the type checker; the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now says “the parameter name belongs to `Type::Fn.params`; this node carries only the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **The reverse bridge and AST→AST reconstruction must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs 7 of items 2–8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 contains 5 variants / 10 sites
> from the 13-item list (2 each for Union/Option/Result/Sum/AssocType). Both categories of
> reconstruction sites must be handled in the same batch as 6.3.1; otherwise, the result is either
> silent behavior changes (see 6.3 #47) or a direct compilation failure.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly under D8
> (item 18 in the 5.2 table). The criterion follows the **parser-forward** direction: zero forward
> construction holds (the syntax layer has no path that produces `::` associated types), so there is
> no need to modify D8 or add it to the whitelist. The **2 production AST→AST reconstruction sites**
> (`function.rs:456`'s `substitute_type_ast` and `function.rs:623`'s `substitute_type_in_ast`) were
> registered in 6.3.2 and rewritten in the same batch as 6.3. The three-category breakdown in the T1
> gate (5.7) reproduces this conclusion (the 6.0.1 commit measured 26/26 variants in agreement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least 4
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine whether something “is a
predicate,” the same file at `510-512` must determine whether it “is an operator interface,”
`src/frontend/core/parser/ast.rs:971` must determine whether it “is an operator interface,” and the
two `name_used_as_type*` implementations—`name_used_as_type_in` in `name_used_as_type_in` (measured
at 864) and `name_used_as_type` at `declarations.rs:42`—must determine whether it “is a type
reference,” all through string comparisons.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target shape (illustrative; line numbers are after the change)
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
must be classified by an injected environment. This connects directly to 5.4.

**Disposition of the synonym table (landed in WBS 6.1.2, 2026-10-27, final user decision)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) is **retained with
downgraded semantics**. Before the change, it was a disambiguator that bridged two equivalent AST
representations; after `Type::Int(64)` was deleted, “Int” has only one representation path, and the
function degenerates into the **sole name→type resolution table**.

Reasons it cannot be deleted:

- It is the **sole resolution point** for standard-library type names and language built-ins (the
  signature strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP list all use it as their
  reference).
- The `DateTime` arm records a real fix (an issue related to #338: `now()`'s return value could not
  be passed to `format_time`)—it is a **semantic alias**, not the same kind of case/abbreviation
  alias as `Int`/`int`, so it is retained separately until P10 S11 (issue #513).

**Disposition result** (final criterion = 15 built-in names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                     | Before                                      | Disposition                                                                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Crowded into 13 arms and mixed with aliases | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                    |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                    | **Delete** (22 spellings); **no lexer alias table**—the corpus measurement found 0 noncanonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | In the same arm as `Void`                   | **Retain**—`()` is the type spelling of `Void` (standard-library signature strings use `() -> Void`), not a different type                                                                               |
| `DateTime`                                                                                                                                                                                                                                   | `datetime` grouped with lowercase aliases   | **Retain only the capitalized** temporary compatibility alias, noting that its migration belongs to P10 S11 (issue #513)                                                                                 |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` and `Void`
share an arm; there is also one temporary `DateTime` alias). After deleting the 22 spellings, a
noncanonical spelling in type position becomes `TypeRef`, and `TypeEnvironment::resolves_type_name`
reports an unknown type—**it is no longer silently accepted**. Previously, `add_builtin_types` also
registered seven lowercase copies (`int` / `float` / `bool` / `string` / `void` / `never` / `char`)
in the type table, amounting to a second alias table; these were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive/negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected `MonoType`
   for each of the 15 names, `None` for the 22 spellings, and `DateTime` → `Int(64)`);
2. **T2.5, the “deleted-spelling sentinel,”** in `tools/type-tables --report`—if any of the 22
   spellings is added back to the resolution table, the report immediately counts it as
   **unregistered drift** (there is no disposition that can be registered: that would be a
   regression).

**Merging the two `name_used_as_type*` implementations**: The two duplicate implementations recorded
in 2.4 must be merged, otherwise the classification information provided by `NameKind` can be
bypassed by two independent string-comparison implementations. The merged implementation is placed
in `parser/ast.rs` (with the data source near `Type`), and `declarations.rs` calls it; the
`is_predicate` short circuit reads `NameKind::Predicate`, and `Struct` / `NamedStruct` field
recursion follows the newer and more complete implementation at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: The const generic parameter name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured at 855-858) partially overlaps the full set in
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured at 932) can become “`kind == Builtin` and the
name belongs to the set of names acceptable for const parameters,” explicitly connecting the two
sources of knowledge; the definition of this const-parameter subset is validated by the T2 gate (see
5.7).

> **Design judgment**: If `.yx` corpus files contain type annotations such as `i64` / `int`,
> normalizing them to `Int` changes those files' **diagnostic positions** (but not diagnostic
> codes). This is permitted under the C3 criterion (messages and ordering may be normalized), but
> the baseline must be recorded in phase 1. **Measurement (P6 6.0, 2026-10-27)**: A full scan of
> `tests/yaoxiang` (316 `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **0 noncanonical
> names in actual type positions**; the 16 raw-token matches were adjudicated individually as 12
> strings + 4 comments (both `i64` / `f64` occurrences were in comments). Therefore, 6.1's name
> normalization **does not change the diagnostic position of any corpus file**, and **no baseline
> regeneration is required**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned using the final **22** spellings—the
> 2026-10-27 count used a **narrow scope** (only the 14 integer/floating-point-family spellings,
> excluding namespace usage in `src/std`), hence the record “16 raw-token matches = 12 strings + 4
> comments.” The retest covered **393 files**: 316 `.yx` files in `tests/yaoxiang` + 39 in
> `tests/yaoxiang-multifile` + 38 in `src/std`, with matches on **308 lines**; there were only **2
> type-position candidates, both inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`'s `i64::MAX` and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`'s `-> f64`) ⇒ **there are still 0 actual
> type positions**; the remaining matches were adjudicated individually as string literals /
> comments / the `std.string` module name and namespace access / a `Json` variant name / a `void`
> literal, with **not one type annotation**. See 6.1.1 in [09](09-execution-wbs.md) and 09a's “P6
> 6.1 name-normalization closure” for the retest command. The conclusion matches 6.0: 6.1's name
> normalization **does not change the diagnostic position of any corpus file**, and **no baseline
> regeneration is required**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed loop inside the parser + 2 out-of-layer references):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injected, without crossing layers):

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

**Changes by file**:

| File                                                                                                                | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                           | Defines `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is legal dependency inversion. **The landed method names are authoritative in this table** (the original design's `is_type_var` / `is_builtin_type` were not adopted: a type variable's identity is determined structurally by its **declaration site**, not by its name, so no such criterion is defined; built-in-name classification is instead provided by `kind_of`, and `knows_builtin_types()` declares whether this probe holds a built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                                    | Retains `declare_predicate` / `is_predicate_name` (for accumulation during the current pass), and adds a `probe: &'a dyn TypeEnvProbe` field plus a `probe()` accessor (**borrow rather than `Box`**). **The landed state also adds a `kind_of()` accessor**—predicates declared in the current pass take priority, and the remainder are delegated to the probe, making this the sole entry point for name classification                                                                                                                                                                                                                                           |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                       | Deletes `n == "Terminates"`; changes it to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                       | Deletes the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; changes it to `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/parser/ast.rs:930`                                                                               | Deletes the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and should not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                                       |
| All **40 parser construction sites** (`ParserState::new` is the sole constructor; **4 production + 36 test sites**) | Accepts a `probe: &'a dyn TypeEnvProbe` parameter. **The default implementation `NullProbe` (all methods return false)** ensures parser unit tests remain unaffected; the production entry point's default probe is `BuiltinProbe` (the compiler's built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (The original design's “5 construction sites” was a pre-survey estimate; the measured value is **40**—`Parser::new` does not exist, and the sole constructor is `ParserState::new`)                                                                                                                                |

**The signature problem in `extract_generic_param_names`**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that receives no context. After the out-of-layer call at `ast.rs:930` is removed, it
needs the `is_constraint` decision, so its signature must gain `probe: &dyn TypeEnvProbe`. Callers
(including `parser/tests/ast.rs`) must likewise pass `&NullProbe`.

**When `NameKind` is classified**: When `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, fixing it once and ensuring that subsequent match arms no
longer perform string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them involve only parser-internal
functions under the 2.4 definition; the genuine cross-layer references are the 2 sites at
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Merging `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Proposition: delete `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This path was **already the running path** (`pratt/nud.rs` constructs
`Expr::Lambda` + `parser/statements/declarations.rs` constructs `StmtKind::Assign`, see 2.6); the
`FnDef` branch was a never-executed parallel implementation.

**Landed state**: The variant definition and all **15 production consumer arms** were **removed in
the same batch** (Rust's exhaustive `match` makes “definition + missing arms” an atomic unit; see
“Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5). The baseline consumption sites are
classified by form as follows:

| Category                                                                       | Count | Disposition                                                                                            |
| ------------------------------------------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------ |
| Variant definition                                                             | 1     | Delete                                                                                                 |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (otherwise compilation fails)                                                                   |
| Non-exhaustive match arms + or-patterns with `_ => Span::dummy()`              | 10    | Delete (after deletion they fall through to the fallback; retaining them would leave a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after deletion they can never match, with neither an error nor a warning)                      |
| Test construction sites                                                        | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not delete the cases)                                         |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally equivalent to the first three fields of `FnDef`; the `name` field unique to `FnDef` has
no use at the `Expr` layer (the name is on `StmtKind::Assign.target`).

**Risk and mitigation (originally planned, now landed)**: The 2 non-exhaustive `if let` sites become
impossible to match after the change, with neither an error nor a warning. Mitigation is
**compile-time assurance**: once the variant is deleted, any subsequent `if let Expr::FnDef` fails
to compile because the variant does not exist, so **no gate is needed**.

**Per-site disposition table, test migration, and measured figures**: See
[09a](./09a-wbs-history.md) §P6 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**`StmtKind::Assign` / `StmtKind::TypeDefinition`: `signature_params: Vec<Param>` → delete.**

Rationale: its reason for existing (the original comment at `ast.rs:244`) was “for typechecker
`classify_generic_params` use”—a **dedicated slot reserved by the statement AST for the type
checker**, redundantly transporting another representation of the parameters.

**The implementation scope differs from this section's initial draft in one important way**
(corrected after phase-1 verification): the initial draft proposed “recomputing the data in place
from `Lambda.params`,” but `Lambda.params` is an **already-filtered** value-parameter list (with
type-position and constraint-position parameters removed), whereas `classify_generic_params` /
`extract_generic_param_names` require the **unfiltered** raw signature list. The actual
implementation therefore first moves the names into the sole carrier, then deletes the old fields:

1. In 6.8.1, first reshape `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   together, reusing the existing `Param`—no second structure and no parallel arrays);
2. In 6.5.2, then delete the two `signature_params` fields: `Assign` obtains them from
   `type_annotation`, and `TypeDefinition` instead carries the declared signature
   `type_annotation: Option<Type>` (the same carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (expanding each
   curry group in order and treating `Paren`/`NamedParen` as transparent); `NamedParen`'s binder is
   not included and is interpreted explicitly by the type checker (see below).

> **Risk-verification conclusion**: The initial draft's concern about the “unfiltered vs. filtered”
> difference was valid—but it was **not a problem**: the flattened result and the old
> `signature_params` are element-for-element co-originative (the same `Param` nodes), so
> generic-classification results do not change. The corpus result of 356/356, zero drift across 216
> IR snapshots, and a fully green `cargo test --all` are the evidence (see
> [09a](./09a-wbs-history.md) §P6 6.5). The initial draft's “use `Lambda.params`” approach would
> change generic-classification results and was **not adopted**.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, remove binder semantics from it
(completed: 6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records its **sufficient reason for existing** (see 2.7): if `r` is
lost from `(r: P(m))`, the type checker can only guess that “a free variable outside the scope of
the constraint is the return-value formal parameter,” and will **silently substitute the undeclared
`m` as a binder as well** (the comment marks this as an “observed defect”).

Therefore, it **cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param`/`param_span`
   carry only source facts and do not bear type-layer binder semantics.
2. Transfer binder interpretation to the **type-check layer**: when the type checker consumes
   `NamedParen`, it **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node's
   field,” analogous to the `Expr` embedded in `ConstExpr`—the syntax node provides facts, and the
   semantic layer interprets them.
3. Rewrite the comment from “the type checker can only guess” to “the parameter-position formal
   parameter name belongs to `Type::Fn.params`; this node carries only the **return-position**
   binder.”

> **Path correction**: The `sema/` in this section's initial draft is the **target** directory name
> from RFC-039 D1; the semantic-layer directory in the current repository is
> `src/frontend/core/typecheck/` (renaming belongs to batch 6.7 and has not yet been performed).
> Code paths in the body of this document consistently use the pre-rename current paths (see
> “Directory Renaming and Paths Used in This Document”).

**Why this is a “migration” rather than a “deletion”**: The syntax tree must represent source code
without loss. `r` genuinely exists in the source, and the AST cannot pretend otherwise. The problem
is not the node's existence, but that **responsibility for semantic interpretation has not been
explicitly defined**.

### 5.7 Build-Time Type-Table Gates

**The reference mechanism has already been verified in this repository**: `build.rs:19-37` invokes
`tools/code-tables`' `parse_registry` / `validate`, performing uniqueness, segment, and
Chinese-completeness checks over 145 error codes, and directly calls `panic!` to reject compilation
if `is_ok()` is false; `build.rs:39-54` further compares the Markdown code table in RFC-013 item by
item and likewise calls `panic!` on a mismatch. **RFC-013's documentation and code therefore always
remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one `serde_json` dependency, and
the validation logic is text-level parsing with no dependency on `syn`.

The type table currently has **no** comparable gate. The mechanism added by this document can
directly reuse that form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). The checks are as follows (the T3 column was removed on 2026-10-30, with the removal
record retained):

| #                                                                   | Check                                                                                                                                                                                                                                                                                                                                                                                                               | Inputs                                                       | Failure condition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Remedy                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site breakdown**                                  | Scan the variant list of `pub enum Type` in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant's construction sites in **three categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`). Reconstruction locations (②③) are driven by the `REBUILD_FUNCTIONS` table in `scan.rs` and reconciled item by item against the source | Variant list + three-category match tables + survey baseline | ① A variant has **zero forward construction** and is not registered; ② a registered variant gains a forward construction site; ③ a registered wildcard-arm fallback exemption no longer matches; ④ an **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered item by item and must not be generalized to every wildcard arm); ⑤ **Survey baseline (T1.6) drift**: a variant's total/forward/AST→AST/reverse-bridge counts differ from `SCOUT_BASELINE` (decision 2A); ⑥ **The reconstruction-location registry differs from the source**: a registered reconstruction function has no matching `fn` in the source (renamed/deleted without synchronization); ⑦ **Criterion and baseline are from different generations**: the current `REBUILD_FUNCTIONS` differs from the frozen criterion snapshot `SCOUT_BASELINE_CALIBER` (criterion revision) (6.6.3) | Delete the variant; or register a rationale / remove an obsolete registration (see 5.2 and `REGISTERED_ZERO_FORWARD`); refresh `SCOUT_BASELINE` with reproducible evidence, including construction-site file:line, when the baseline drifts; changing the criterion requires refreshing the criterion snapshot and baseline figures in the same batch; keep the registry synchronized with the source |
| **T2 Synonym table and repository-wide name-list consistency**      | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:855` + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                                                                                                                                                            | Three tables                                                 | A type name exists in one table but is not recognized by another (compare according to each table's semantic mapping rules); **from WBS 6.1.2, there is also T2.5, the “deleted-spelling sentinel”: any of the 22 deleted spellings added back reports a failure**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | `type-tables --fix` reconciles in both directions                                                                                                                                                                                                                                                                                                                                                     |
| ~~**T3 Built-in type names consistent with the RFC-011 document**~~ | Originally planned: the canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                                                                                                            | —                                                            | **Removed following the user ruling of 2026-10-30 (the comparison target does not exist)**: RFC-011 is a generic type-system design and contains no table with “built-in type names” as its columns                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | — (not applicable; see the explanation after the table for the removal rationale and coverage)                                                                                                                                                                                                                                                                                                        |
| **T4 Exhaustiveness of `Type → MonoType`**                          | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                                                                                                                                                      | Two tables                                                   | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Add an arm / delete an arm                                                                                                                                                                                                                                                                                                                                                                            |

> **T3's removal and T2's coverage scope (user ruling of 2026-10-30)**: T3's **comparison target
> does not exist**—RFC-011 is a generic type-system design, and its body contains no table whose
> columns are “built-in type names”; the first implementation had to use “all tokens in the body of
> every Markdown table that are wrapped in inline code and are overall capitalized identifiers” as
> the document-side criterion, and the result was inevitably a meaningless “no action required”
> shell discrepancy (16 registrations: 3 document-only + 13 code-only, none of which could be
> adjudicated). Therefore, this check, its 16 registrations, and the corresponding output sections
> in `--report` / `--json` were **removed in the same batch**, with no placeholder left in the code.
>
> **There are only three code-side carriers of built-in type names, all reconciled name by name by
> T2**: ① `from_builtin_name` (`src/frontend/core/types/mono.rs`; 15 canonical names + the temporary
> `DateTime` compatibility alias, with the first spelling in each arm being the canonical name; the
> 22 in-implementation spellings deleted in 6.1.2 are guarded by the T2.5 sentinel); ②
> `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`; semantically a **subset of ①**, check C1);
> ③ the LSP built-in type-name list (`src/lsp/world.rs`; semantically a **superset of ①**, checks C2
> / C3). No equivalent list is established on the documentation side: the authoritative landing
> point for a new built-in name is ①, and T2 reports gaps in ②③ as **unregistered drift**.

**T4 implementation note**: Rust's exhaustive match already guarantees the “variant → arm” direction
at compile time. T4's value lies in the **reverse** direction—detecting stale arms in `lower.rs`
that point to deleted variants (this code already fails to compile when a variant is deleted, so T4
is in practice redundant). **T4 is therefore downgraded to a CI assertion** (it passes when
`cargo build` succeeds) and does not enter `build.rs`'s `panic!` path. **T1/T2 are the two items
that genuinely require text parsing** (T3 was removed, see above).

**T1's failure mode (6.6.2, user decision 2A of 2026-10-30)**: T1 has changed from “report only” to
a **hard build failure**—`build.rs` invokes `type_tables::build_gate(root)`: `Ok(summary)` prints
one `cargo:warning` line (an informational line, not a violation), while `Err(details)` directly
calls `panic!` to reject compilation. **All six violation classes use the same failure path** (see
①–⑤ and ⑦ in the table above; ⑥ belongs to the scanning-layer failure path “the
reconstruction-location registry differs from the source,” and, like the different-generation
criterion introduced in 6.6.3, directly reports failure but is not counted as a gate-violation
class). Each error includes the **variant name** and the construction-site / fallback-site /
variant-definition `file:line`; baseline drift also includes these locations (following F1 in the
6.6.3 review, which previously provided only numbers, leaving readers unable to determine where to
look). Failure to complete the scan itself (file parse failure or incomplete criterion) also counts
as failure and **must not be downgraded to “pass.”** **The registration mechanism is retained with
empty tables**: `REGISTERED_ZERO_FORWARD` and `DEAD_VARIANTS_03` are now both empty (after 6.3
deleted the 13 dead variants, no zero-construction variants remain); T1.7's “registered but no
longer valid / exemption did not match” alarms and injectable verification (`*_with(registry)`)
continue to work—any newly added zero-construction variant must be deleted or have its rationale
registered, or the build fails.

**Single source of truth for reconstruction locations (6.6.2, user decision 1A)**: The
classification of the second category and reverse bridges is driven by the `REBUILD_FUNCTIONS` table
in `scan.rs` (**file + function name → category**). `Type::without_fn_param_names` (the
fidelity-preserving reconstruction introduced in batch 6.5: it recreates an existing AST type to
strip type-position parameter names, with its sole consumer normalizing it before entry into the IR
via `ir_gen.rs`) belongs to the **second category, AST→AST reconstruction**; previously it was
counted as parser-forward, artificially increasing the forward count of 11 variants by 1 (`Name` /
`ConstExpr` use `self.clone()` and do not produce sites). The registry is reconciled **item by
item** against the source: if a registered function has no matching `fn` in the source, the scan
reports an error directly and **must not silently fall back to the forward category** (otherwise,
after a rename or deletion, reconstruction sites would masquerade as construction sites).

**The baseline numbers pin down two things (6.6.3 review F1/F2)**:

1. **The source tree**—`SCOUT_BASELINE_REV`: point counts are comparable only for the same source
   tree;
2. **The criterion**—`SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` (= a frozen snapshot of
   `scan.rs::REBUILD_FUNCTIONS` at that revision, recorded as “file + function → category key”). The
   snapshot **does not share a table with the registry**, specifically so that a criterion change
   becomes detectable: if they shared a table, both would change together and the gate would never
   see the criterion change.

The gate performs a machine reconciliation of the snapshot and the current registry
(`baseline::caliber_drift`, modeled on `scan::missing_rebuild_functions`): adding/removing a
reconstruction function or changing a function's category reports “**the criterion and baseline are
from different generations**” (with the criterion revision and item-by-item differences) and
**skips** baseline-drift comparison—point counts across criteria are not comparable, and continuing
to compare them would disguise “the criterion changed” as “the numbers drifted.” The body of T1.6 in
the `--report` output prints the baseline revision and **criterion revision**, allowing readers of
the report or scripts to reproduce both.

**T1's two implementation challenges and mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` also has only
   `serde_json`. Mitigation:
   - **Preferred**: add `syn` to `tools/type-tables` (that crate only; the root crate remains
     unchanged). `syn` is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` line-level heuristic (an existing
     Python tool in the repository)—construction sites are on the right-hand side of `=` or in
     function-argument position, whereas match arms are on the left-hand side of `=>`; approximate
     the distinction using the number of `=>` occurrences.
   - **Conservative starting point (completed in this order)**: When T1 first shipped, it **reported
     without failing** (`cargo:warning`), taking one full baseline to confirm that the heuristic was
     sound before changing it to `panic!`—it reported only from 6.0.1 and **changed to `panic!` in
     6.6.2 (2026-10-30)**; by the time it became hard, the baseline had already been refreshed under
     the same criterion to agree on all 13/13 rows. This is consistent with handling the risk that
     “a new CI gate may initially be red; establish a baseline first.”
2. **Reverse bridges and AST→AST reconstruction sites must be classified separately**. Section 2.2
   records the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; 2.2.2 records the AST→AST reconstruction in the two replacement functions
   in `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites from the 13-item list).
   If T1 conflated them with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` /
   `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` would be classified as
   “having construction sites” and exempted from deletion, directly contradicting the disposition
   table in 5.2. T1's report must list the three categories separately, and **fallback exemptions
   apply only to itemized registrations** (currently only the parser error fallback `types.rs:836`
   for `Void`), never to every wildcard arm.

**Gate coverage (landed in 6.6.2, user decision 2A of 2026-10-30)**: `build.rs` invokes
`type_tables::build_gate(root)` immediately after the `code-tables` validation block, reusing the
`panic!`-to-reject-compilation form—`Ok(summary)` prints one `cargo:warning`, while `Err(details)`
calls `panic!("Type-table gate T1 failed (build rejected): …")`. **All six violation classes use the
same failure path**: zero-construction variant not registered / registered variant gains a forward
construction site / registered exemption site does not match / unregistered wildcard-arm fallback
site / **criterion and baseline are from different generations** / **survey baseline (T1.6) drift**;
each error includes the variant name and the construction site, fallback site, or variant-definition
`file:line` (drift entries have included construction-site locations since 6.6.3). Failure to
complete the scan itself (such as a file parse failure) also counts as failure and **must not be
downgraded to “pass.”**

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                                                                |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                  |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `check_structural_termination` at `478` in `eval/dependent_types.rs`, and `unify` at `reducer.rs:572` all operate on `MonoType`                                                                                                                                                                                                             |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. Eight sites in `mono/function.rs`—`358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`—match `AstType::NamedStruct` / `AstType::AssocType`; `AssocType` was designated for deletion, and all 8 sites were synchronized. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3, using `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match converged with 6.3; the former direct read of `signature_params` as `let params: Vec<MonoType>` now flattens through `Type::fn_params_flat` (the verification conclusion in 5.6: an unfiltered list co-originative with the old field)                                                                                         |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs modification**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                                                                                          |
| Interpreter (`backends/interpreter/`)                        | **Needs modification**. `image.rs:44`'s `type_table: Vec<ir::Type>` changes to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects the `{:?}` output format                                                                                                                                                                  |
| RFC-027 compile-time types                                   | **Needs modification**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                                                                                                |

**IR-layer type discipline (to be inherited by 04-ssa)**: After convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by the “type consistency” invariant of the first-layer validator in 07.
`ast::BinOp::Assign` / `ast::UnOp::Deref` (2.5) are analogous expression-side forms—they may occur
inside `ConstExpr`, and therefore must likewise be prevented from descending into IR type
annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (`Void`'s
   single forward site was an error fallback—**WBS 6.3 changed that fallback to `Type::Name{"()"}`
   and removed the gate-exemption registration**, so there are now no forward construction sites) →
   the parser does not produce them (the reverse-bridge and AST→AST reconstruction dispositions are
   covered by #28 / #47 in 6.3 and the 10 sites in 6.3.2, all implemented).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs—this **may**
   change the behavior of some programs (types that were previously lost are now preserved). **This
   is the only direction of behavior change, and it is a fix rather than a regression**, but it must
   be confirmed to cause no regressions by the third-layer corpus differential in 07.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Preserve the true `MonoType::Generic`          |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Preserve the true `MonoType`                   |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match is sufficient) |

> **Design judgment**: Because all 8 variants currently collapse to `Void`, **every code path that
> depends on signature bytecode (the interpreter's arity/type checks and serialization format)
> currently sees `Void`**. After convergence, these locations see the true types. **This is the only
> possible source of behavioral change in this document, and its direction is “restoring correct
> information.”** If the corpus differential produces a difference, it should be treated as
> **exposing an existing defect**, not as a regression introduced by this document—the method for
> adjudication is to construct a `.yx` program containing a `Ref` parameter and compare
> `dump_bytecode` before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                        | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Category                          |
| --- | ---------------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`      | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**Completed: WBS 6.3.1, 2026-10-29**—all 13 variants were deleted, and `ast::Type` converged to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                   | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original location, **deleted**)    | Delete `Expr::FnDef` (**Completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consumer arms were deleted in the same batch, with 0 references in `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                   | Delete `Assign.signature_params` (**Completed: WBS 6.5.2, 2026-10-30**—the identically named field was deleted from `TypeDefinition` in the same batch, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm location, **deleted**) | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3 (completed)            |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                   | Change the `NamedParen` comment to “syntax provides the binder name; semantic interpretation occurs in the **type-check layer**” (**Completed: WBS 6.5.2, 2026-10-30**—the comment became “the parameter name belongs to `Type::Fn.params`; this node carries only the return-position binder”; the `sema/` in the initial draft is D1's target directory name, whereas the repository's current directory is `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                           | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                             | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Addition                          |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                         | `trait TypeEnvProbe` + `NullProbe` (**Completed: WBS 6.4.1, 2026-10-29**—the landed state also includes `BuiltinProbe` and `NameKind`; the trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Addition                          |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                     | Add the `probe` field and accessors (**Completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites are wired**: 4 production + 36 test sites)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                   | Use `state.probe()` to determine `NameKind` (**Not done**: 6.4 introduced only the **classification enum + probe queries** and did **not** change the structure of `Type::Name`—fieldizing it would affect every production/test construction site and require the reverse bridge `mono_to_ast_type` to preserve `kind` during write-back, necessitating regeneration of the IR snapshots and corpus baseline and conflicting with this batch's C3 surface (zero drift); fieldizing `NameKind` is deferred until a second consumer appears)                                                                                                                                                                                                                                                                                                                             | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                   | Update the comment (`Result`/`Option` use `Generic`, which already matches the facts; only add a note about the variant deletion)—**Not done**: 6.4 did not modify `types.rs` (the change surface in this file belongs to 6.3's fallback refactoring), and this line is deferred together with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                       | Delete the `_ => (Vec::new(), Type::Void)` fallback and use an explicit error return—**WBS 6.3.1 changed the fallback to a let-else explicit return of `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical); **the “use an explicit error return” half was not done**—it was originally deferred to 6.4's error-path refactoring, but **6.4 did not take over that path** (6.4 only injected type-name dependencies and did not modify `types.rs`) ⇒ **6.5 did not take it over either** (that batch only reshaped representations and left the fallback path unchanged) ⇒ **it moved to 6.6**, where it was registered in 09's 6.6.1 row. The path is syntactically reachable (the `Name` arm of `semantic_tokens` records one additional dummy-span token; there are 0 corpus matches and zero drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                   | Delete `n == "Terminates"` and use `state.probe().is_predicate(n)` (**Completed: WBS 6.4.3, 2026-10-29**—the landed state reads `NameKind::Predicate` through `ParserState::kind_of`: predicates declared in the current pass take priority, and the rest are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                   | Delete the `typecheck::operator_interfaces::spec()` call (**Completed: WBS 6.4.3, 2026-10-29**—changed to `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                       | Delete the `signature_params` argument—**Completed (WBS 6.5.2, 2026-10-30)**: all construction sites deleted the field; in the same batch, the HashMap name table in the name-merging section was changed to positional alignment (the criterion is that prior validation guarantees identical names at every position)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                        | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #3 (completed)            |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                       | Retain (the parameter-list retrieval point)—**the “change to read Lambda directly as in 5.5” was not done**: 6.5.1's change list did not include this item, and its disposition is to be decided together with 6.8.2's declaration-driven validation mode                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`      | Delete 13 match arms (one for each deleted variant) (**Completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                   | Remove synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Convergence                       |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                       | Retain the `DateTime` alias                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                         | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Addition                          |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                               | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Convergence                       |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`)   | Delete `pub use ... ast::Type` (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                       | Keep `FunctionCode.params` as `Vec<MonoType>` (already the case)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Convergence                       |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                       | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Convergence                       |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                      | Change the `type_table` conversion to `.collect()` (the elements now have the same type)—the actual change is to **delete the conversion and transport the values unchanged** (**Completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)          | **Delete** the entire `From<MonoType> for IrType` impl (**Completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                        | Change `type_table` to `Vec<MonoType>` (**Completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Convergence                       |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                       | Handle the `AstType::AssocType` arm according to the gate conclusion—**the conclusion has been established and implemented: WBS 6.3.2 deleted all 5 variants / 10 sites registered in 03 §2.2.2** (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each), rewriting both bridges (`substitute_type_ast` / `substitute_type_in_ast`) in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                | Type-table gates T1/T2 (the original T3 was removed following the user ruling of 2026-10-30)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                       | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                       | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                      | Same, but obtain the name from `Assign.target`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                              | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                        | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                       | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                               | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                   | Delete the `Expr::FnDef` test (**Completed: WBS 6.5.1, 2026-10-30**—the cases were not deleted; they were rewritten as two Lambda-destructuring cases + one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                   | Rewrite as Lambda + Assign (**Completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`             | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                   | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be handled in the same batch as #1**—**the criterion has been corrected (Completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes the lowercase spellings, it would resolve to `TypeRef` and **silently lose type identity**; the actual implementation therefore uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                           | Convergence                       |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` (**Completed: WBS 6.4.3, 2026-10-29**—the landed signature is at `ast.rs:927`; the 3 production callers in `ir_gen.rs` pass `SemanticProbe`, while the rest pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                               | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                     | Delete the local `name_used_as_type` and call the merged implementation in `ast.rs`; the `is_predicate` short circuit now reads `NameKind::Predicate` (**Completed: WBS 6.4.4, 2026-10-29**—the single merged implementation is at `ast.rs:848` and covers header classification + predicate short circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                       | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3) (**Completed: WBS 6.4.5, 2026-10-29**—dual-source wiring: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe that does not hold the built-in-name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Convergence                       |

> **The 6 landed rows in 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed on
> 2026-10-28) implemented these 6 rows in one batch—four carriers changed to carry `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType`
> and the `ir::Type` re-export alias were deleted (`ir.rs:3`'s `pub use` became a private `use`),
> and decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`). **The line
> numbers for these rows have been reset to their landed positions** (the original anchors `812` /
> `814` / `854` / `2339` / `2352-2390` were pre-change positions). See [09a](./09a-wbs-history.md)
> §P6 6.2 for measured figures and gate results.

> **The gate must itself be gated**: `tools/type-tables`, like `code-tables`, needs a `mod`
> declaration for wiring (this repository has previously had 8 orphan test trees). Register the new
> crate in the root `Cargo.toml` when it is created.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/` (293 single-file `.yx` corpus files) + the multi-file corpus layer
established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library **must
all retain their behavior** (same diagnostic codes and messages; ordering may be normalized).

| Risk                                                                      | Assessment                                                                                                                                                                                                                                                                                                                               | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains type annotations such as `i64` / `int` / `f64`       | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                                        | **Measured** (end of 5.3): 0 noncanonical names in actual type positions → 6.1.2 directly deleted the 22 spellings and **did not establish a lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Source code contains `DateTime`                                           | Only the capitalized temporary compatibility alias is retained (`mono.rs:883`); it is not a built-in name                                                                                                                                                                                                                                | Migrating it out of the built-in-name table belongs to P10 S11 (issue #513); until then, `DateTime` remains synonymous with `Int`, with no behavior change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `("()")` occurs in type position                                          | It is not a product of the `Name` form, but the type spelling of `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                                            | **Retain**: `()` shares an arm with `Void` and is one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `dump_bytecode` output changes                                            | The element type of `type_table` changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                                              | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only during C1/C2). The output diff must nevertheless be recorded in phase 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Behavior changes after the lossy bridge is fixed                          | See 6.2                                                                                                                                                                                                                                                                                                                                  | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| The type-replacement result changes after `mono_to_ast_type` is rewritten | `function.rs:507-529` is the exit point for monomorphization type replacement (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting the variants, taking the `_` arm would change the replacement result from `AstType::Int(64)` to something such as `AstType::Name { name: "i64" }` | Targeted phase-3 comparison: select corpus files containing generic substitution and compare the monomorphization products' `type_name()` before and after the change. **Do not assume that `{:?}` output is equivalent**. **The criterion has been corrected (measured in phase 3)**: equality must not use `type_name()`—it is the lowercase internal name used for diagnostic display, and compared with `type_name()` alone, one can only see that both are “`int64`,” not whether the canonical name is `Int64` or `i64`; instead, adjudicate by **canonical-name round trip** (`from_builtin_name` ↔ `MonoType::canonical_name()` are inverses for every name) + structural comparison of the write-back product. The WBS 6.3 measurement found that the dumps and `.42` products of 4 generic-substitution programs were byte-for-byte identical to their pre-change forms |
| `.yx` source code itself                                                  | **Zero modifications**                                                                                                                                                                                                                                                                                                                   | This document does not require changes to any corpus or std source file. This is a hard acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

**Hard acceptance condition** (**criterion revised on 2026-10-27, following decision ② of
2026-10-26**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly
driven test files** under `tests/` may be modified when mechanically required by the changes; each
must be registered with its **file / line count / rationale**, and **tests must not be deleted or
assertions weakened**. The original criterion (`git diff --stat tests/ src/std/`) imposed a
zero-diff requirement on all of `tests/`; this revision relaxes the requirement by one level for
directly driven test files only, while the zero-diff requirements for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`) remain
unchanged. **Exception**: the 6.8.2 behavior-fix batch is a separate round with a separate ruling.

## Implementation Notes

All criteria use **C3 from 07: the same diagnostic codes and messages (ordering may be
normalized)**. The following is the stage breakdown for this document's workstream; see RFC-039 for
the global ordering across documents.

| Stage                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Rollback point                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2 (T3 landed in stage 0 and was later removed following the user ruling of 2026-10-30). The first version of T1 **reports without failing**. Take a full baseline on **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | The T1 report accurately lists the 13 **parser-forward zero-construction** variants (including the classification of `AssocType`: 0 forward / 2 AST→AST; D8 unchanged), and **separately lists the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; 13 parser-forward zero-construction variants and 1 globally zero-construction variant (`Enum` only) are reported separately; T2 is fully green or provides a repair list | No code changes; no rollback required                                                                                                                                                                                                                                                                                                                                                                          |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) converged to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; the 7 lowercase copies in `add_builtin_types`, the LSP list's `Uint` / `Int128`, and the `sizeof` `Uint` arm were cleaned up in the same batch; **no lexer alias table was established** (the corpus measurement found 0 noncanonical names). **Also verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**Verified: it uses the unfiltered raw signature list; after 6.5.2 it is provided by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                                                                    | C3: the complete corpus diagnostic set is **identical item by item** to the stage-0 baseline                                                                                                                                                                                                                                                                                                                                                                                | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                    |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers changed to carry `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy bridge `From<MonoType> for IrType` (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias (`ir.rs:3`'s `pub use` became a private `use`) were deleted; decoding now transports values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), and the silent information loss in which 8 `MonoType` variants collapsed to `Void` no longer occurs                                                                                                                                                                                                                                                                                                          | C3 + targeted `dump_bytecode` comparison (item by item in the 6.2 table)—**measured fully green** (see [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                 | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                    |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: delete 13 variants according to the 5.2 table (`ast::Type` 26 → 13); clean up match arms such as `mono.rs:699-708` and the `types.rs:836` fallback (changed to a let-else explicit return of `Type::Name{"()"}`); **handle `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; synchronize `types/tests/mono.rs`. **The criterion for #47 has been corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” becomes invalid after 6.1—`type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 deletes the lowercase spellings it would resolve to `TypeRef` and **silently lose type identity**; the actual implementation uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural write-back | C3 + a fully green `cargo build` (exhaustive matches force completion of every arm) + targeted generic-substitution comparison (last row of 6.4)—**measured fully green** (see [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                         | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                    |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: add `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); modify `parser_state.rs` (add the `probe` field and the `probe()` / `kind_of()` accessors; **wire all 40 construction sites** = 4 production + 36 test); delete the 2 parser-side `operator_interfaces::spec()` calls and change the 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); merge the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connect `CONST_PARAM_TYPES` through `NameKind::Builtin` (#50). **Not done**: add the `kind` field to `Type::Name` at `types.rs:162-165` (#10; fieldizing it conflicts with this batch's C3 surface) and change the fallback at `types.rs:836` to an explicit error return (#12, moved to 6.5)                                                               | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**measured that both greps (`typecheck` / `operator_interfaces`) have 0 matches**; corpus 356/356, zero drift across 216 IR snapshots, and `cargo test --all` exit 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                        | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                                    |
| **5. Dead AST variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: delete `Expr::FnDef` (baseline measurement: **25 sites / 16 files** = 1 definition + 15 production consumption sites + 3 test construction sites + 6 comments, of which 19 are code references) + delete the two parallel parameter tables in `Assign` / `TypeDefinition`; reshape `Type::Fn.params` into `Vec<Param>` (names and types together) → the sole flattening entry point `Type::fn_params_flat`; retain `NamedParen` as a syntax node and have the type checker explicitly interpret the binder                                                                                                                                                                                                                                                                                                                                               | C3 + all **22 modules** in `tests/integration/` green (measured 22 `.rs` files; the original document's 18 was an old count); formatter parameter-position output byte-for-byte unchanged (byte-for-byte comparison across all 355 `.yx` corpus files); corpus 356/356 and 216 IR snapshots with zero drift; `#[test]` count in `src/` 2878 → 2883 (no tests deleted)                                                                                                       | 7 commits (2 variant-deletion commits + 3 representation-reshaping and old-representation-deletion commits + 2 fix commits); **revert policy**: the variant-deletion commit can be reverted independently, but the test-migration commit must be reverted with it (see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5); see [09a](./09a-wbs-history.md) §P6 6.5 for measured figures and gate results |
| **6. Gates become hard**                      | **Completed (6.6.2, user decision 2A of 2026-10-30)**: T1 changes from warning to `panic!`, and **T1.6 survey-baseline reconciliation is connected to the same failure path** (construction-site counts that differ from `SCOUT_BASELINE` fail); at the same time, decision 1A moves `without_fn_param_names` into the second category (restoring the forward count to the frozen value); T4 is downgraded to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | `cargo build` **must fail** when a zero-construction variant is deliberately introduced; changing any `SCOUT_BASELINE` variant count to 1 **must also fail**—both mutations were measured using mechanically reversible patches (`scripts/dev/scratch.py trial`)                                                                                                                                                                                                            | Separate commit                                                                                                                                                                                                                                                                                                                                                                                                |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage 2**—before stage
2 lands, the bytecode-side reverse bridge A (`bytecode.rs`'s `From<MonoType> for IrType`) consumes
`IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation; that
bridge was deleted in 6.2.2, and the **remaining rewrite surface when stage 3 deletes the variants
is reverse bridge B** (`middle/passes/mono/function.rs`'s `mono_to_ast_type`, which still resolves
`String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`. Stage 3's #47
must be handled in the same batch as #1; otherwise, `mono_to_ast_type` silently changes behavior.
**Stage 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B was changed to use
`canonical_name` + structural write-back; the three `ast::Type` carriers in `middle/core/ir.rs`
**were not changed**—6.3 only deleted variants and did not alter the IR shape (216 IR snapshots and
the 356-line corpus baseline had zero drift), and carrier disposition was left to stage 4.

**Downstream dependency**: [04-ssa.md](04-ssa.md) may begin only after all stages in this document
are complete. At that point, the element types of `BytecodeFunction.params`, `FunctionCode.params`,
and `type_table` are consistent, giving SSA type annotations a single landing point.

### Mandatory Validation for Each Stage

| Stage | Must execute                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction sites as a separate category**)                                                                                                                                                                                                                                                                                                                              |
| 1-5   | `cargo test`; full corpus differential over `tests/yaoxiang/` (07 third layer); item-by-item C3 comparison                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                                              |
| 3     | Additionally: for corpus files containing generic substitution, compare the monomorphization products' `type_name()` before and after the change—**the criterion has been corrected (measured in stage 3)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient for equality; instead, adjudicate by **canonical-name round trip** (`from_builtin_name` ↔ `MonoType::canonical_name()` are inverses for every name) + structural comparison of the write-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red→green)                                                                                                                                                                                                                                                                                                                                                                                                   |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                        | Rationale                                                                                                                                                                                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                                                                                                                                         | The alias at `ir.rs:3` directly causes the field-type mismatch between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After deleting the alias, the two field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                       | It would create an L2→L3 reverse dependency (violating RFC-039's layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See 5.1 for the complete argument                                                                    |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and migrate binder semantics to the **type-check layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is D1's target directory name, while the current repository directory is `typecheck/`) | `ast.rs:518-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as the binder                                                                             |
| Directory rename             | **Do it** (D1), as a pure-move batch at the end of P5/P6; paths in this document's body use the pre-rename current paths                                                                                                                          | Decision of 2026-10-03 (ChenXu233); implementation method clarified on 2026-10-04                                                                                                                                                           |

### Rejected Alternatives

None of the four alternative approaches was adopted: **retaining all three representations and
adding only a conversion layer** resolves none of the root causes, and `ir::Type` remains a
non-independent type while none of the dead variants, synonym table, or reverse dependencies
disappear; **using a macro to generate the synonym table** treats the symptom rather than the
cause—after `Type::Int(usize)` is deleted, the macro has no reason to exist, and consistency should
instead be guaranteed by the T2 gate (more verifiable than macro-expansion correctness); **deleting
only the dead variants without unifying** has the lowest cost but leaves the problems in place and
provides no gate to prevent the next person from adding another `Type::Int128(usize)` and recreating
the same dead-variant problem (if only one commit can be made, it should explicitly be recorded as a
subset of this approach and include delivery of the T1 gate); **unifying on `MonoType`** is
addressed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. `Expr::FnDef`'s 15 production consumption
  sites (including 3 exhaustive arms) + 3 test construction sites + 1 variant definition form a
  never-executed parallel path; after deletion, `ir_gen.rs` / `ownership.rs` / `termination.rs` each
  lose one branch.
- **Eliminates one duplicated implementation that has diverged**. The two copies of
  `name_used_as_type` (2.4) already disagree about field recursion within definition bodies; after
  merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value.”** This is a
  prerequisite for the “type consistency” invariant of the first-layer validator in 07 to pass.
- **The gates make regression impossible**. T1 turns “adding a zero-construction variant” from
  silently passing into a build failure—the existing `build.rs` mechanism in this repository
  demonstrates that this path is feasible.
- **Zero source-code changes**. The 293 `.yx` corpus files and std library require no modifications,
  which is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may change behavior**. After the lossy bridge is fixed, 8 `MonoType` variants expose
  their true types rather than `Void`. The direction is corrective, but it changes code paths that
  depend on signature bytecode. This is the document's largest uncertainty.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify. This is why phase 0 requires manual
  verification.
- **If T1 fails to count reverse bridges, the disposition table and gate contradict each other**.
  Section 2.2 proves that 7 “dead variants” have production reconstruction sites in the reverse
  bridge. If T1 does not separate construction-site directions, those 7 variants will be classified
  as “having construction sites” and exempted from deletion—the gate will become an obstacle to the
  deletion work rather than a safeguard. | **2. Deleting `signature_params` has semantic risk**
  (**verified and resolved: 6.5.2, 2026-10-30**). It carries the **unfiltered** parameter list,
  whereas `Lambda.params` is **filtered**. If `classify_generic_params` depends on the former, the
  change would alter generic-classification results. **Measured conclusion**:
  `classify_generic_params` / `extract_generic_param_names` require the unfiltered list; flattening
  `Type::Fn.params` yields a list co-originative with the old `signature_params` (the same `Param`
  nodes), so the classification results do not change. The “use `Lambda.params`” approach would
  change behavior and was **not adopted** (corpus 356/356, snapshots 216, and `cargo test --all`
  fully green). | **3. Stage 3 touches `middle/passes/mono/function.rs`**. Apart from the 8
  `AssocType` match sites, `mono_to_ast_type` at `507-529` is the exit point for monomorphization
  type replacement, and rewriting it may change the replacement result's `type_name()` (last row of
  6.4).
- **The corpus might contain noncanonical type names**. If the corpus extensively uses `i64` /
  `int`, stage 1's “directly delete synonyms” would be infeasible and would have to fall back to
  “move them into a lexer alias table,” reducing the scope of `from_builtin_name`'s convergence.
  (**Measured as 0**: the 6.0 survey at the end of 5.3 found none, so WBS 6.1.2 directly deleted the
  aliases without taking the alias-table route.)
- **The gate may initially be red** (**this risk was handled in two steps**). If T1 directly used
  `panic!` on its first deployment, it would immediately block compilation; therefore, it first
  reported only from 6.0.1 to establish a baseline, and changed to `panic!` in 6.6.2; by the time it
  became hard, the baseline had already been refreshed under the same criterion to agree on all
  13/13 rows, and the current tree has 0 zero-construction variants.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document leaves no pending items.**

## See Also

### Upstream and Peer Documents

- [RFC-039 Compiler Feature Routing Directory Design (Master Blueprint)](../../rfc/accepted/039-compiler-architecture.md)
  — upstream master blueprint; four-layer model, criterion tiers, routing table C (the 3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — Feature routing tables A/B/C, dependency-direction rules, target
  directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document's prerequisite and a downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; convergence of the
  operator-change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, and the type-consistency invariant of the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Generic mechanism
  and type-level constructors (`If` / `Match`); **does not carry a built-in type-name list**, so it
  is not a gate-comparison target (the original T3 was removed following the user ruling of
  2026-10-30, see 5.7)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of a build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line anchors were reset to the 6.5 landed state** (2026-10-30); entries deleted by earlier
> batches are retained and annotated with their deletion batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after the 6.3
  closure; the 26-variant era spans 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — `Type::NamedParen` binder-semantics explanation
  (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types together) and
  `Type::fn_params_flat` (flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- `src/frontend/core/parser/ast.rs:858` — `Type::without_fn_param_names`: strips type-position
  parameter names before an IR carrier enters the IR (closing the printing-boundary consistency gap,
  see 5.6)
- `src/frontend/core/parser/ast.rs:1074` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single implementation after the
  6.4 merge (the original `name_used_as_type_in` and the copy in `declarations.rs` were merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through the injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents all primitive types
  as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: fallback for non-`Fn`
  forms (since 6.3, returns `Type::Name{"()"}`; the `Type::Void` variant was deleted)
- (**Deleted in 6.4**) `src/frontend/core/parser/statements/declarations.rs:42-80` — the original
  `name_used_as_type`, merged into the single implementation in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call (changed in 6.4 to use `ParserState::kind_of` / the
  injected probe; both literal references have disappeared, leaving only this comment)
- `src/frontend/core/parser/pratt/nud.rs:512` — expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — the `expr_to_params` arm retrieves the parameter
  list
- `src/frontend/core/types/mono.rs:863` — the `from_builtin_name` built-in-name resolution table (15
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
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: the reverse bridge rewritten in 6.3
  (using `canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — the interpreter module's `type_table`
- `build.rs:33-51` — error-code registry build-time gate (the model for the type-table gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; `tools/type-tables` introducing `syn` is the first exception)
