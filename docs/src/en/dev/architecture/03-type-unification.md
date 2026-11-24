# Unifying Type Representations

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the RFC-039 main text for the four-layer model, tiered acceptance criteria, and execution-stage
> ordering; see the [index of this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

### Covered

Consolidate the multiple parallel type representations currently in use into one representation,
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero forward construction in the parser** (among the 26 variants in
  `ast.rs:427-541`, see 2.2; the “forward” definition excludes reverse bridges and AST→AST
  reconstruction sites)
- **A handwritten table of type-name synonyms** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser’s genuine reverse dependency on the type-checking layer** (2 sites, see 2.4)
- **One expression variant, `Expr::FnDef`, with zero production construction** (**removed: WBS
  6.5.1, 2026-10-30**—the variant definition and all consuming arms have been removed; see 2.6)

The concrete deliverables are four items: a per-variant disposition table for all 26 variants (5.2),
semantic classification of `Type::Name` through `NameKind` and elimination of the synonym table
(5.3), injected data flow through `TypeEnvProbe` (5.4), and build-time type-table gates T1/T2 (5.7;
the original T3 was deleted following the user ruling of 2026-10-30).

### Directory Rename and This Document’s Path

RFC-039 decision D1 mandates a directory rename (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`)
to be completed **together with P5/P6, without staging**. This document’s 50-item change list and
all acceptance greps are written against the **pre-rename paths**—they are the construction baseline
for the P6 batches **before the rename batch**. The directory rename at the end of each batch is a
**pure move batch** (C1 zero-diff) and is committed separately. The baseline becoming invalid after
the rename commit is expected; use git history as the bisection basis.

### Not Covered

- **How the directory rename is performed**. During this phase, `Type` does not move out of
  `parser/ast.rs`; only `lower.rs` is added under `types/` as the single conversion point. The
  directory rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 according to
  RFC-039 decision D1; see the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction specification**. See
  [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and validator implementation**. See
  [07-equivalence-oracle.md](07-equivalence-oracle.md) for the definitions of C1-C6 and the three
  tiers of criteria (IR validator / canonicalized snapshots / corpus differential). This document
  cites only **C3 (type-representation convergence)** from that set as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reasons
  given below.
- **The global P1-P10 execution sequence**. See the RFC-039 main text and
  [02-stage-contract.md](02-stage-contract.md). This document provides only the stage breakdown for
  its own workstream (see “Implementation Highlights”).

### Division of Labor with RFC-039

RFC-039 is the overarching blueprint: it defines the four-layer model, criterion tiers,
cross-document stage ordering, and the rationale for undertaking this refactoring now. This document
is the construction plan for **type-representation convergence**, one workstream within RFC-039:
which variants to remove, which files to change, in what order, and which gates prevent regression.

All line numbers, variant lists, and construction-site counts in this document come from static
verification of `src/`, with each item cited as `file:line`. **This document records no execution
process; it records code facts and the dispositions derived from them.**

For convenient cross-references with documents from the same batch, the “Target Design” and
“Detailed Design” chapters retain their `5.x` / `6.x` subsection numbering; all other chapters
follow this directory’s standard structure.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(the `pub use` line at the head of `src/middle/core/ir.rs` before 6.2.2; that alias has since been
removed and is therefore no longer cited by line number). [04-ssa.md](04-ssa.md) is to introduce
register type annotations and new instructions into the IR. Without converging the representation
first, SSA type annotations would follow the `ir::Type` alias directly onto the syntactic type with
26 variants, effectively institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:505`) embeds a complete expression tree in a type. It
  is impossible for an `Expr` to appear in an IR instruction’s type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:475-481`) carries a source name and `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently harden the 13 dead variants
and the two syntax-specific variants into the IR contract.

## Current State

Everything below is a statement of code fact, not a proposal. Each item provides `file:line`
evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 landed)**: this section records the state **before** WBS 6.2
> landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed four type carriers to carry `MonoType`, removed
> the lossy `From<MonoType> for IrType` bridge and the `ir::Type` re-export alias; every statement
> below concerning `ir::Type` or the lossy bridge is therefore a **pre-change fact**, and entries
> marked “removed in 6.2.2” identify dispositions now in effect. See [09a](./09a-wbs-history.md) §P6
> 6.2 for implementation details and measured results.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                              | Causal relationship to type representation                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple legal AST forms                                 | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, and the parser chooses the latter, type checking must add a table to translate the former back                                                                                   |
| Bytecode-layer signatures differ from IR-layer signatures                       | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias; consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places, with no checks | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; both locations duplicate the mapping and do not validate each other                                                                                                                        |
| The parser hard-codes `"Terminates"`                                            | Because the parser cannot determine whether a name “is a type” or “is an operator interface,” it can only embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                         |

**The first three items are representation problems; the fourth is a downstream side effect of a
representation problem.** Doing only item 1 (removing dead variants) would leave the synonym table
and reverse dependency in place; doing only item 4 (injecting a callback into the parser) would
build on an still-inconsistent representation.

One of the three parallel representations is an alias illusion:

| Name        | Definition site                                                            | Size        | Role                                                    |
| ----------- | -------------------------------------------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541`                                  | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`                                          | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | Before 6.2.2: head of `src/middle/core/ir.rs` (that line has been removed) | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

Before 6.2.2, the head of `ir.rs` first contained one `pub use … parser::ast::Type;` line,
immediately followed by one `use … typecheck::MonoType;` line. The IR module depended on two “type”
symbols at once, although one was only an alias. Both lines disappeared when the alias was removed
and are therefore no longer cited by line number.

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was removed in 6.2.2).** `From<MonoType> for IrType`
(formerly at `src/middle/core/bytecode.rs:2353-2390`) downgraded `MonoType` to `ast::Type`, and its
**information loss was silent**:

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

All eight `MonoType` variants collapsed into `IrType::Void`, in addition to the `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; the comment explicitly describes it as a “transitional branch
before deleting enum variants”).

Here, `IrType::Void` is a **legal type value, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function’s return type was lost
during bridging”—one reason why the first-layer “type consistency” invariant of 07 cannot currently
pass on this code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), precisely the dead variants that 2.2 will remove—**both ends of
the lossy bridge point to dead code**.

**Current state (from 6.2.2)**: the impl has been **removed in its entirety**, and decoding no
longer downgrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly carry
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now copies the values unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those
eight `MonoType` variants retain their real types in signatures and type tables, and `IrType::Void`
is no longer used as a substitute for “lost information.” The `ir::Type` re-export alias (the
`pub use ... ast::Type` line at the head of `src/middle/core/ir.rs` before 6.2.2) has also been
reduced to a private import—the IR layer no longer publicly re-exports a syntax-layer type. The IR
layer previously had three carriers that directly held `ast::Type` (`Instruction::Cast.target_type`
/ `Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`): the first two were removed in
**WBS 6.6.11** together with two **zero-producer** dead variants (their payloads were already
discarded downstream); the last was replaced in **WBS 6.6.12** with the IR-native narrowed carrier
[`TypeDeclBody`] (field table `Vec<TypeDeclField>` plus bytecode type-table entry
`self_type: MonoType`). **The IR layer no longer holds any `ast::Type`, and `src/middle/core/ir.rs`
no longer imports the syntax-layer `ast`.** The **bytecode-side `BytecodeInstr::Cast` of the same
name is a separate construct** (its payload is a `u16` type-table index, not a syntactic type) and
remains unchanged.

### 2.2 Parser-Forward Zero-Construction Variants

**Measurement scope**: the construction-site scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. A **construction site** is a `Type::<variant>` path in expression position (pattern
positions do not count), and sites are reported in **three separate categories that must not be
combined**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Among the 26 variants of `ast::Type`, the following 13 have zero construction in the
**parser-forward** direction (reconstruction sites in the latter two categories are covered by 2.2.1
/ 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                                    |
| ------------------------------------ | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                  |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the sole forward site = error fallback** `_ => (Vec::new(), Type::Void)`, individually registered as an exemption; eliminated with 6.4 #12)                                                                                                                                                        |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                                  |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                       |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                       |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                       |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                       |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 remains unchanged** |

> **Definition-column line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the
> “Definition” column in this table and the table in 5.2 was written on 2026-10-05. Since then, 17
> lines were inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so the
> definition lines of **all 26 variants shifted uniformly by +17** (for example, `Int` moved from
> 432 → 449 and `NamedParen` from 533-540 → 550-557). This round only **registers** the drift and
> does not alter the table body; subsequent references use the `ast::Type` symbol as the anchor.

The causal chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   states explicitly: “RFC-010: Result/Option are not lowered into dedicated AST nodes… They follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The sole forward construction site of `Type::Void` is an error fallback**—`types.rs:836`
   contains `_ => (Vec::new(), Type::Void)` at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`; they are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document uses the term **“zero construction in the parser-forward direction”**: it
means neither “zero references globally” nor “zero production construction.” Eleven of these
variants do have production construction sites in the other two categories (reverse bridge / AST→AST
reconstruction). These tests themselves codify the behavior under test: they assert exactly what
these syntactic forms lower to as `MonoType`, even though the parser never emits them. When deleting
the variants, these tests must be deleted or rewritten accordingly.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The “zero forward construction” statement in the table above holds only in the **parser → AST
forward-construction** direction. In the reverse direction, there are two production-level
`MonoType → ast::Type` reconstructions that **actually construct seven of the variants in the
table**:

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

- The `bytecode.rs` site disappears with #28 of 6.3 (deleting the entire `From<MonoType> for IrType`
  impl), so it adds no extra cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name decoder (annotated `#299` in the comment), converting `String` / `Bytes` from `Generic`
  back into dedicated variants by name. After deleting `ast::Type::String` / `Bytes`, these two
  guard branches must be rewritten; after deleting `Int` / `Float` / `Bool` / `Char` / `Void`, the
  behavior of the `_` arm changes (some types that previously fell into `_` will silently move to
  `AstType::Name`). This is the 47th supplementary item of 6.3.
- **Direct consequence for the gate**: a T1 scan that considers only forward construction sites
  would classify these seven variants as having construction sites and thus exclude them from the
  removable list. T1 must explicitly include the reverse bridge in its measurement scope; otherwise,
  the gate’s conclusions contradict the disposition table in this document (see 5.7).

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

**When 6.3 deletes these five variants, it must rewrite these 10 sites in the same batch** (09’s
6.3.2 already registers them); otherwise, these arms fail to compile after the variants are deleted.
**Implemented (WBS 6.3.2, 2026-10-29)**: all 10 sites were removed; see the disposition table in
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
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent at the AST layer but
structurally different (and the parser emits only the latter), `from_builtin_name` must accept case
variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"` on the
`Void` line is especially telling: it works with the `Type::Void` fallback at `types.rs:836`.

The same knowledge—which names are built-in type names—exists in seven copies throughout the
repository, none of which are checked for consistency:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, and the original line numbers have drifted)           | Form                                                                                                                                                                                                                              |
| --- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings** after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                                   |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                          | String match, copying `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                                          |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                        | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                          |
| 4   | `src/lsp/world.rs:182`                                                                                                           | LSP-side list of built-in type names (14 = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                                    |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                          | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` family of aliases (registered by the P0 gate on 2026-10-06)                                             |
| 6   | Base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                          | String match, name → byte size. This previously included a `"Uint"` arm (item 1 had no `Uint` mapping); WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **Dead arm and live evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                   | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set except by name” rule (`inference/expressions.rs:2917-2918`, decision #300-4)                                                  |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223 and the others by 2–9 lines), but **the
> item counts and contents have not changed** (item 1 had 13 arms when this section was written,
> item 3 has 15 entries, and item 4 had 15 entries when written; after WBS 6.1.2, item 1 has 15 arms
> / 16 spellings and item 4 has 14 entries). Subsequent references use **symbol anchors + repository
> paths**; the line numbers in this table are for location only. The arm count of item 1 and the
> live drift evidence in items 5 / 6 / 7 (the dead `Uint` arm and the contradiction between `Set`
> and the typecheck side) **remain**, and this section did not change code.

`CONST_PARAM_TYPES` in item 3 is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to determine const generic parameters. It
enumerates names usable as const parameters and has a **partially overlapping but different
relationship** to the full set handled by `from_builtin_name`, with no mechanism ensuring
synchronization. Item 4 is a fourth copy used for LSP syntax highlighting and hover; it likewise has
no validation (and T2’s comparison scope includes it).

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 / 7 share the same root as items 1–4
(string-keyed tables mapping built-in names to semantics / IDs), though they serve different
purposes and carry the same drift risk. The same gate also found 11 string dispatches of built-in
container names on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4,
`"String" | "Bytes"` ×4, etc.). These are **neither within this section nor within the parser
data-flow scope of 5.4**; the inventory and disposition are registered in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as a P6 extended verification item.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both of which the table treats as type-layer
functions). **Measurement differs from that description**: both functions are defined inside the
parser itself, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the
  repository)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the only `fn` definition in the
  repository)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production-code sites** that cross the layer boundary into `typecheck`, and
both call the same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                             |
| --- | --------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Determines constraint-position parameters (`T: Add`), which do not occupy runtime parameter slots   |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Determines constraint-position parameters and produces `GenericParamName` values with `constraints` |

The second site is more deeply embedded: **`parser/ast.rs` itself** (`ast.rs:847-948`, the entire
block alongside `StmtKind` / `Expr` in the same file) directly references a typecheck symbol.
Routing table C records only the `declarations.rs` site.

Together with the hard-coding at `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Expanded as more built-in predicates are added (future open-set approach: pass predicate names down to
// the parser, or use a form decidable at the syntax layer).
// The parser does not hold a type environment, so it checks two sources: hard-coded built-in predicates,
// plus declarations accumulated during the current pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment explicitly acknowledges this as a temporary form for a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask “is this an
operator-interface name?” because it must distinguish whether `Type::Name` refers to a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings** (concrete type / type variable / constraint name / predicate name), so the
syntax layer cannot determine the answer itself and the authority is pushed upward into the type
layer.

#### Duplicate Implementation: Same Name and Meaning, but Behavior Has Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` explicitly
acknowledges this:

```rust
/// Whether a parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; independently implemented here to avoid
/// a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                                          | `ast.rs:847-885` `name_used_as_type_in`                     |
| ----------------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                              | Takes no callback                                           |
| `Type::Generic` branch  | First short-circuits through `is_predicate(app_name)` and returns `false` (`:54-56`) | Has no such short circuit and directly recurses into `args` |
| `Type::Struct { body }` | **Absent** (falls through to `_ => false`)                                           | **Present** (`:871-879`, traverses `Field` / `Expr`)        |
| `Type::NamedStruct`     | **Absent**                                                                           | **Present** (`:880-882`, traverses `fields`)                |

The comment at `ast.rs:868-870` records the origin of the latter two branches: “These two branches
were previously missing; when a const generic parameter was referenced in a definition body, it
could not be determined that it was ‘used as a type,’ and annotation validation incorrectly reported
the valid const parameter as an unknown name.”

In other words: **the function was duplicated to avoid a cross-module dependency, but subsequent
fixes did not flow back to the original.** The `ast.rs` version recursively traverses fields in a
definition body, while the `declarations.rs` version does not—the same source construct yields
different conclusions at two decision points. This duplication must be consolidated when 5.3
introduces `NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums that do not reference each other and **whose members have already
diverged**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                    |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer uniquely has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer uniquely has `Deref` (`:222`); the constant layer uniquely has `BitNot` (`:329`)                                             |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` in `const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**. When they occur in type position, they fall
into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly related to
the IR-layer type discipline introduced in 6.1: after convergence, `Type::ConstExpr` disappears once
`Type → MonoType` has been applied and is never visible to the IR, while `Assign` / `Deref` continue
to exist through paths such as `ir_gen.rs`’s `BinOp::Assign => Ok(MonoType::Void)`
(`inference/expressions.rs:875`). The divergence between the two enums means that “which operators
may occur in type position” cannot be determined independently from either side.

This phase does not merge the two enums (their ownership is addressed in “Changes Affecting
Operators” in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate’s scan targets
should include `ast::BinOp` / `ast::UnOp`, making newly introduced zero-construction operators
visible as well.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction (Removed: WBS 6.5.1, 2026-10-30)

**This section records a fact that has already landed**: `Expr::FnDef` used to be a variant of
`ast::Expr`. Its **zero production construction** was accurate—there were only two construction
sites in the entire repository (`src/frontend/core/parser/tests/ast.rs` and
`src/frontend/core/typecheck/tests/checker.rs`), both in tests.

However, **there were far more than four consumption sites**: the baseline measurement found **15
production consumption sites** (classified in 5.5), plus 6 comments mentioning the variant. In the
final 6.4 state, `git grep -n "FnDef" -- src/` measured **25 lines / 16 files** = 1 definition + 15
production consumption sites + 3 test construction sites + 6 comments.

**The path actually used for function definitions** was: `pratt/nud.rs` constructs `Expr::Lambda`,
and `parser/statements/declarations.rs` constructs `StmtKind::Assign` (placing the `Lambda` in
`value`). In other words, the `FnDef` branch was **a parallel path that never executed but was
maintained by 19 code locations** (15 consumption sites + 3 test construction sites + 1 definition).

**Landed state**: the variant definition and every consuming arm were **removed completely**;
`git grep -n "Expr::FnDef" -- src/` within `src/` returns 0 matches (the bare word `FnDef` remains
only in 2 explanatory comments). See [09a](./09a-wbs-history.md) §P6 6.5 for the per-site
disposition table, test migration, and measured results; see 5.5 for the design of the
consolidation.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` once carried a typechecker-specific field (fixed: WBS 6.5.2 + 6.8.1,
2026-10-30).** Before 6.5.2, the `Assign` variant contained `signature_params: Vec<Param>`
(originally `ast.rs:244-245`), and its comment stated plainly: “The first parameter group of the
signature is retained verbatim (including parameter names) for the typechecker’s
classify_generic_params to use”—the statement AST reserved a dedicated slot for the type checker.

Current representation: parameter names and types **co-reside** in the type annotation’s
`Type::Fn.params` (`Vec<Param>`), whose sole access point is `Type::fn_params_flat` (`ast.rs:985`).
`Assign` no longer stores a separate parameter table; `TypeDefinition` now carries the declaration
signature as `type_annotation: Option<Type>` (the same carrier as `Assign`).

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The arm at `src/frontend/core/parser/pratt/led.rs:466`,
`Expr::Lambda { params, .. } => Some(params.clone())`, then “retrieves” it and treats it as a
parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, after which
another module’s match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:518-532` states explicitly:

> RFC-027 §3’s return-position refinement relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable in a constraint that is not in scope is the return-value parameter,” so `(r: P(m))`
> silently substitutes the undeclared `m` as a binder as well (observed defect).

A syntax node is storing the binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is demoted to the type checker’s internal working representation; the `ir::Type` alias is
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

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), while
   `MonoType` does not. C3 in 07 requires identical diagnostic codes and messages—diagnostic
   positions must remain traceable to source code. Adding `Span` to `MonoType` would leave
   monomorphized types carrying a mass of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   in `types/solver.rs:411`). These are **intermediate states in the type checker’s solving
   process**, not types themselves. Putting them in the sole type structure would require the
   IR-construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counter-argument**: elevating `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer’s type. RFC-039 explicitly forbids L2 from
   depending on L3. One of this document’s goals is to eliminate reverse dependencies; selecting
   such a design would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) caused
`BytecodeFunction.params` (`bytecode.rs:812`) and `return_type` (`bytecode.rs:814`) to use
`ir::Type`, while the serialization side used `MonoType` (`codegen/bytecode.rs:277-278`). The two
field types were connected through `bytecode.rs:2339`’s
`file.type_table.into_iter().map(|t| t.into())` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed in full,
eliminating the reverse reconstruction recorded in 2.2.

**Explicitly out of scope**: moving `Type` from `parser/ast.rs` to `sema/types/`. This phase only
adds `lower.rs` under `types/` as the sole conversion point and leaves the definition of `Type` in
place; the directory rename is performed at the end of P6 as an independent move batch according to
D1—at that point, `Type` will live in the top-level AST domain at `ast/type_.rs`, while
`sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> **Implemented (WBS 6.3, 2026-10-29)**: all 13 “delete” items in this table **have been
> implemented**—`ast::Type` was reduced from 26 variants to 13 (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were **rewritten in the same batch**. See [09a](./09a-wbs-history.md) §P6 6.3 for
> measured results, the 10-site disposition table, and the criteria.

> Disposition categories: **delete** (zero construction in the parser-forward direction), **retain**
> (has a forward construction site), **migrate** (semantics belong to another node).

> **Line-number convention**: the “Definition line” column contains the variants’ **original
> positions before deletion** (the 26-variant era; 6.3 deleted 13 of them); the matching-site line
> numbers in the “Rationale” column likewise use the **pre-6.3 baseline** and do not move with
> subsequent code changes. This batch only refreshed the definition lines for the **still-existing
> variants** (rows 14 and 26) and corrected the now-invalid consumption site in row 14.

| #   | Variant                                                            | Definition line  | Disposition                           | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthened)             | The parser’s **sole** primitive-type exit (`types.rs:162-165`). After deleting the 12 primitive-type variants, every primitive type uses it. It needs a new annotation for the “semantic kind” of the name (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:699` is a match arm). Bit width is carried through the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372`’s `IrType::String` consumes it, and that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:703-706` is a match arm). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                            | The sole parser-forward construction site is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, individually registered as an exemption). That fallback is eliminated together with the 6.4 error-path refactoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                            | Has production consumption sites (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                            | Has a production construction site at `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:743-746` is a match arm); 2 AST→AST reconstruction sites (`function.rs:377` / `606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:747` is a match arm); all three categories total 0 (the sole globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; this is unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                            | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:463-466` | **Retain**                            | Consumed by `src/frontend/core/types/mono.rs:960` (`From<ast::Type> for MonoType`) and `src/formatter/handlers/types.rs:36` (type-position rendering); the original reference to `bytecode.rs:2366` was to `IrType::Fn` and disappeared with the deletion of the `ir::Type` alias and lossy bridge in 6.2. **Field reshaping `params: Vec<Type>` → `Vec<Param>` (names and types co-located) is part of 6.8.1 (D54)**—`Assign.signature_params` names are carried over accordingly (see 6.5.2). **Completed (WBS 6.8.1, 2026-10-30)**: `Param` gained `anonymous` / `from_type` / `is_named`; anonymous parameters (pure type positions) are synthesized with an empty name, so this table’s “net effect” is unchanged; `Param` has a hand-written `Debug` implementation (anonymous parameters print as their carried type, producing zero IR-snapshot drift; see 5.6) |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427` / `600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that the `Generic` path is used                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431` / `602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                            | The actual representation of `Result` / `Option` / `String` / `Bytes` uses it (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                            | Zero construction in the parser-forward direction (the syntax layer has no `::` associated-type path); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 remains unchanged**: the criterion is forward construction; if associated-type syntax is enabled in the future, a new proposal at that time will define it                                                                                                                                                                                                                                                                                                                                                                                             |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                            | Zero construction in the parser-forward direction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467` / `632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                            | Has a production construction site at `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                            | Raw pointer type inside unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                            | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                            | Has production construction sites at `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain** (marked compile-time-only) | Has a production construction site at `types.rs:160`. It is the only variant in `Type` that embeds an `Expr`, and the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                            | RFC-004’s curry layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                           | The semantics (binder name) move to the type checker; the AST retains only syntax. See 5.6. **Completed (WBS 6.5.2, 2026-10-30)**: the node is retained; `checker/refinement.rs` explicitly extracts `param` as the binder; the `ast.rs` comment now says “parameter-position names belong to `Type::Fn.params`; this node carries only the return-position binder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **The reverse bridge and AST→AST reconstruction must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs 7 of items 2–8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 contains 5 variants / 10 sites
> within the 13-item list (2 each for Union/Option/Result/Sum/AssocType). Both categories must be
> handled in the same batch as 6.3.1; otherwise, behavior either changes silently (see #47 in 6.3)
> or the code fails to compile.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly according to
> D8 (item 18 in the 5.2 table). The criterion is **parser-forward construction**: zero forward
> construction holds (the syntax layer has no path that emits an associated type using `::`), so D8
> need not change and the variant does not enter the whitelist. **The 2 production AST→AST
> reconstruction sites** (`function.rs:456`’s `substitute_type_ast` and `function.rs:623`’s
> `substitute_type_in_ast`) have been registered in 6.3.2 and are rewritten in the same batch as
> 6.3. The three-category breakdown of the T1 gate (5.7) reproduced this conclusion (all 26 variants
> matched in the 6.0.1 commit).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine “is this a predicate?”,
lines 510-512 of the same file must determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` must determine “is this an operator interface?”, and the two
`name_used_as_type*` implementations (`name_used_as_type_in`, measured at 864 in `ast.rs`, and
`name_used_as_type` at 42 in `declarations.rs`) must determine “is this a type reference?”—all
through string comparisons.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target form (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // added
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → delegated to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: classifying `NameKind` requires exactly the information the parser cannot
currently obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, the field **cannot be classified independently inside the parser**—it
must be classified by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 implemented, user-approved final wording on
2026-10-27)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:1121-1143`) **is retained, with
reduced semantics**. Before the change, it disambiguated two equivalent AST representations; after
`Type::Int(64)` was deleted, “Int” had only one representation path, reducing the table to the
**sole name→type resolution table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language built-ins
  (`src/std/*.rs` signature strings, `CONST_PARAM_TYPES`, and the LSP list all refer to it).
- The `DateTime` arm records a real fix (related issue #338: the return value of `now()` could not
  be passed to `format_time`). It is a **semantic alias**, not the same kind of alias as the `Int` /
  `int` case variants, and is therefore retained separately until P10 S11 (issue #513).

**Disposition result** (final criterion = 15 built-in names; the temporary `DateTime` alias was
moved out by S11.1 (issue #513)—the bare name now reports E1003/E1002):

| Category                                                                                                                                                                                                                                     | Before                                           | Disposition                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Mixed into 13 arms with aliases                  | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                       |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                         | **Deleted** (22 spellings), with **no lexer alias table**—corpus measurement found 0 non-canonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | Shared arm with `Void`                           | **Retained**—`()` is the type spelling of `Void` (standard-library signature strings use `() -> Void`), not another type                                                                                    |
| `DateTime`                                                                                                                                                                                                                                   | Shared arm with `datetime` and lowercase aliases | Moved out (S11.1, issue #513—the alias arm was removed; the bare name reports E1003/E1002)                                                                                                                  |

**Net effect**: `from_builtin_name` = **14 match arms / 15 spellings / no aliases** (`()` and `Void`
share an arm; the temporary `DateTime` alias was moved out by S11.1 (issue #513)). After deleting
the 22 spellings, non-canonical spellings in type position become `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—**they are no longer silently
accepted**. Previously, `add_builtin_types` also registered seven lowercase spellings—`int` /
`float` / `bool` / `string` / `void` / `never` / `char`—in the type table, constituting a second
alias table; those were removed in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/mono.rs` (the expected `MonoType`
   for each of the 15 names; the negative case already asserts
   `from_builtin_name("DateTime") == None`—`DateTime` no longer maps to `Int(64)`);
2. **T2.5, the “deleted spelling sentinel,”** in `tools/type-tables --report`—if any of the 23
   strings (the 22 spellings deleted in the 6.1.2 final wording plus the `DateTime` moved out by
   S11.1) is added back to the resolution table, the report immediately counts it as **unregistered
   drift** (there is no disposition to register: that would be a regression).

**Consolidating the two `name_used_as_type*` implementations**: the duplicate implementations
recorded in 2.4 must be merged into one; otherwise, the classification information provided by
`NameKind` would be bypassed by two independent string-comparison implementations. The merged
implementation belongs in `parser/ast.rs` (next to the `Type` data), and `declarations.rs` should
call it. The `is_predicate` short circuit should read `NameKind::Predicate`, and `Struct` /
`NamedStruct` field recursion should use the newer, more complete implementation at
`ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps the full set handled by
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the determination in
`extract_generic_param_names` (same file, measured 932) can be changed to “`kind == Builtin` and the
name belongs to the set accepted for const parameters,” explicitly connecting the two sources of
knowledge; the const-parameter subset definition is validated by the T2 gate (see 5.7).

> **Design judgment**: if a `.yx` corpus file contains a type annotation such as `i64` or `int`,
> canonicalizing it to `Int` changes the **diagnostic position** in those files (but not the
> diagnostic code). This is permitted under C3 (messages and order may be canonicalized), but the
> baseline must be recorded in stage 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of
> `tests/yaoxiang` (316 `.yx` files), `tests/yaoxiang-multifile`, and `src/std` found **0
> non-canonical names in actual type positions**; 16 raw-token matches were adjudicated individually
> as 12 strings + 4 comments (both occurrences of `i64` / `f64` were in comments). Therefore, 6.1’s
> name canonicalization **does not change the diagnostic position of any corpus file**, and **the
> baseline need not be regenerated**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned the **22 spellings** defined in the final
> wording—the 2026-10-27 result used a **narrow scope** (counting only the 14 integer/floating-point
> family spellings and excluding namespace usage in `src/std`), hence the original “16 raw-token
> matches = 12 strings + 4 comments.” The retest covered 393 files: 316 `.yx` files in
> `tests/yaoxiang`, 39 in `tests/yaoxiang-multifile`, and 38 in `src/std`, with matches on **308
> lines**; only **2 lines were type-position candidates, and both were inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`, `i64::MAX`, and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`, `-> f64`) ⇒ **actual type positions
> still number 0**; all other matches were individually adjudicated as string literals / comments /
> the `std.string` module name and namespace access / the `Json` variant name / the `void` literal,
> with **not one type annotation**. See the 6.1.1 entry in [09](09-execution-wbs.md) and “P6 6.1
> Name Canonicalization Closure” in 09a for the retest command. The conclusion is consistent with
> 6.0: 6.1’s name canonicalization **does not change the diagnostic position of any corpus file**,
> and **the baseline need not be regenerated**.

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

**Target data flow** (injection, with no boundary crossing):

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

| File                                                                                                                | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                           | Defines `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—a legitimate dependency inversion. **The implemented method names follow this table** (the original design’s `is_type_var` / `is_builtin_type` were not adopted: a type variable’s identity is determined structurally by its declaration site, not by its name, so no such criterion is needed; built-in-name classification is provided by `kind_of`, and `knows_builtin_types()` declares whether this probe holds the built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                                    | Retains `declare_predicate` / `is_predicate_name` (for accumulation during the current pass), adds a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**borrowed rather than boxed**). **The landed state also adds a `kind_of()` accessor**—predicates declared during the current pass take priority, and the remaining names are delegated to the probe, making this the sole entry point for name classification                                                                                                                                                                                                               |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                       | Removes `n == "Terminates"`; changes it to `state.probe().is_predicate(n)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |     | state.is_predicate_name(n)` |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                       | Removes the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; changes it to `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/parser/ast.rs:930`                                                                               | Removes the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                    |
| All **40 parser construction sites** (`ParserState::new` is the sole constructor; **4 production + 36 test sites**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. The **default implementation, `NullProbe` (all queries return false),** keeps parser unit tests unaffected; the production entry point’s default probe is `BuiltinProbe` (the compiler’s built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (**The original design’s “5 construction sites” was a pre-survey estimate; measurement found 40**—`Parser::new` does not exist, and `ParserState::new` is the only constructor)                                                                                                                       |

**Signature issue in `extract_generic_param_names`**: `ast.rs:891`’s
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` is a free function
that receives no context. After removing the boundary-crossing call at `ast.rs:930`, it needs the
`is_constraint` determination, so its signature must add `probe: &dyn TypeEnvProbe`. Callers,
including `parser/tests/ast.rs`, must pass `&NullProbe` accordingly.

**When `NameKind` is classified**: when `types.rs:162-165` constructs `Type::Name`, it uses
`state.probe()` to determine `kind`, fixes it once, and ensures that all subsequent match arms no
longer perform string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 already concerned only
parser-internal functions under the scope of 2.4, while the genuine cross-layer references are the 2
sites at `declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Consolidation of `Expr::FnDef` (Completed: WBS 6.5.1, 2026-10-30)

**Proposition: delete `Expr::FnDef`; function definitions uniformly use `Expr::Lambda` +
`StmtKind::Assign`.** This path was **already the active path** (`pratt/nud.rs` constructs
`Expr::Lambda`, and `parser/statements/declarations.rs` constructs `StmtKind::Assign`; see 2.6),
while the `FnDef` branch was a never-executed parallel implementation.

**Landed state**: the variant definition and all **15 production consuming arms** were **removed in
the same batch** (Rust’s exhaustive `match` makes “definition + added arms” an atomic unit; see
“Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5). Baseline consumption sites are classified
by form as follows:

| Category                                                                       | Count | Disposition                                                                                         |
| ------------------------------------------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------- |
| Variant definition                                                             | 1     | Delete                                                                                              |
| Exhaustive match arms (`Expr::span()` / `expr_end_line()` / `get_expr_span()`) | 3     | Delete (otherwise compilation fails)                                                                |
| Non-exhaustive match arms + `_ => Span::dummy()` or-patterns                   | 10    | Delete (after deletion, control falls to the fallback; retaining them would leave a silent surface) |
| Non-exhaustive `if let`                                                        | 2     | Delete (after deletion, they never match and produce neither an error nor a warning)                |
| Test construction sites                                                        | 3     | Rewrite as `Expr::Lambda` + `Assign` (do not delete the cases)                                      |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally matching the first three fields of `FnDef`; `FnDef`’s unique `name` field has no usable
location at the `Expr` layer (the name resides on `StmtKind::Assign.target`).

**Risk and mitigation (originally planned, now implemented)**: the 2 non-exhaustive `if let` sites
become impossible to match after the change, without an error or warning. Mitigation is
**compile-time assurance**: after the variant is deleted, any new `if let Expr::FnDef` fails to
compile because the variant no longer exists, so **no gate is required**.

**Per-site disposition table, test migration, and measured results**: see
[09a](./09a-wbs-history.md) §P6 6.5.

### 5.6 Disposition of `signature_params` and `NamedParen` (Implemented: WBS 6.5.2 + 6.8.1, 2026-10-30)

**`StmtKind::Assign` / `StmtKind::TypeDefinition`: `signature_params: Vec<Param>` → delete.**

The reason for its existence (the original comment at `ast.rs:244`) was “for the typechecker’s
`classify_generic_params` to use”—a **dedicated slot reserved by the statement AST for the type
checker**, redundantly transporting a second representation of the parameters.

**The implementation approach differs in one important respect from this section’s initial draft**
(corrected after stage 1 verification): the draft proposed “recomputing `Lambda.params` in place,”
but `Lambda.params` is the **filtered** value-parameter list (constraint-position and type-position
parameters removed), while `classify_generic_params` / `extract_generic_param_names` require the
**unfiltered** original signature list. The actual implementation therefore moves the names into the
sole carrier before deleting the old field:

1. 6.8.1 first reshapes `Type::Fn.params` from `Vec<Type>` to `Vec<Param>` (names and types
   co-located, reusing the existing `Param`—no second structure and no parallel arrays);
2. 6.5.2 then deletes the two `signature_params` fields: `Assign` obtains them from
   `type_annotation`, and `TypeDefinition` now carries the declaration signature as
   `type_annotation: Option<Type>` (the same carrier as `Assign`);
3. The sole access point, `Type::fn_params_flat`, flattens the `Type::Fn` chain (curry groups expand
   in order; `Paren` / `NamedParen` are transparent); the `NamedParen` binder is not included and is
   explicitly interpreted by the type checker (see below).

> **Risk-verification conclusion**: the initial draft’s concern about the “unfiltered vs. filtered”
> difference was valid—but **it is not a problem**: the flattened result and the old
> `signature_params` are element-for-element from the same source (the same `Param` nodes), so
> generic classification does not change. The evidence is 356/356 corpus files, 216 IR snapshots
> with zero drift, and a fully green `cargo test --all` (recorded in [09a](./09a-wbs-history.md) §P6
> 6.5). The initial draft’s “use `Lambda.params`” approach would have changed generic-classification
> results and **was not adopted**.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but move binder semantics out (completed:
6.5.2, 2026-10-30).**

The comment at `ast.rs:518-532` records the **sufficient reason** for its existence (see 2.7): if
`r` is lost from `(r: P(m))`, the type checker can only guess that “a free variable in a constraint
that is not in scope is the return-value parameter,” and will **silently substitute the undeclared
`m` as a binder as well** (the comment marks this as an “observed defect”).

Therefore, it **cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. `NamedParen { param, param_span, inner }` is retained as a **pure syntax node**; `param` /
   `param_span` carry source facts only and do not bear type-layer binder semantics.
2. Interpretation of the binder moves to the **type-checking layer**: when consuming `NamedParen`,
   the type checker **explicitly** extracts `param` as the binder (at
   `src/frontend/core/typecheck/checker/refinement.rs:421`). This is “reading a syntax node’s
   field,” the same kind of operation as the `Expr` embedded in `ConstExpr`—the syntax node supplies
   a fact, and the semantic layer interprets it.
3. The comment changes from “the type checker can only guess” to “parameter-position formal names
   belong to `Type::Fn.params`; this node carries only the **return-position** binder.”

> **Path correction**: the `sema/` path in this section’s initial draft is the **target** directory
> name from RFC-039 D1; the semantic-layer directory in the current repository is
> `src/frontend/core/typecheck/` (the rename belongs to batch 6.7 and has not yet been performed).
> All code paths in this document are written according to the pre-rename current state (see
> “Directory Rename and This Document’s Path”).

**Why this is a “migration” rather than a “deletion”**: the syntax tree must represent source code
without loss. `r` genuinely exists in the source, so the AST cannot pretend otherwise. The problem
is not the node’s existence, but the **failure to define the responsibility for semantic
interpretation explicitly**.

### 5.7 Build-Time Type-Table Gates

**The reference mechanism has already been validated in this repository**: `build.rs:19-37` invokes
`parse_registry` / `validate` in `tools/code-tables` to validate uniqueness, section ranges, and
Chinese completeness for 145 error codes; when `is_ok()` is false, it directly executes `panic!` to
reject the build. `build.rs:39-54` further compares the code table entry by entry against RFC-013’s
Markdown table and likewise executes `panic!` on any mismatch. **RFC-013 documentation and code
therefore always remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one `serde_json`
dependency, and the validation logic is text-level parsing with no dependency on `syn`.

The type tables currently have **no** comparable gate. The mechanism added by this document can
directly reuse that pattern.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). The checks are as follows (the T3 column was deleted on 2026-10-30, but the deletion
record is retained):

| #                                                                    | Check                                                                                                                                                                                                                                                                                                                                                                                                              | Inputs                                                    | Failure condition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Remedy                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site breakdown**                                   | Scans the variant list of `pub enum Type` in `src/frontend/core/parser/ast.rs`; under `src/`, counts each variant’s construction sites in **three categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`). Reconstruction locations (②③) are driven by the `REBUILD_FUNCTIONS` table in `scan.rs` and reconciled line by line with the source | Variant list + three-category hit table + survey baseline | ① A variant has **zero forward construction** and is not registered; ② A registered variant has acquired a forward construction site; ③ A registered wildcard-arm fallback exemption no longer matches; ④ An **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered individually and must not be generalized to all wildcard arms); ⑤ **Survey baseline (T1.6) drift**: a variant’s total / forward / AST→AST / reverse-bridge counts differ from `SCOUT_BASELINE` (decision 2A); ⑥ **The reconstruction-location registry disagrees with the source**: a registered reconstruction function has no same-named `fn` in the source (renamed or deleted without synchronization); ⑦ **Caliber and baseline are from different generations**: the current `REBUILD_FUNCTIONS` differs from the frozen-caliber snapshot `SCOUT_BASELINE_CALIBER` (caliber revision) (6.6.3) | Delete the variant; or register a rationale / remove the stale registration (see 5.2 and `REGISTERED_ZERO_FORWARD`); refresh `SCOUT_BASELINE` with reproducible evidence if the baseline drifts (including construction-site `file:line`); when changing the caliber, refresh the caliber snapshot and baseline numbers in the same batch; keep the registry synchronized with the source |
| **T2 Synonym table and repository-wide name-list consistency**       | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `sema/probe.rs` (moved back from `parser/ast.rs` to L3 by WBS 6.6.9) + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                                                                                                   | Three tables                                              | A type name exists in one table but is not recognized by another (compared according to each table’s semantic mapping rules); **from WBS 6.1.2, T2.5 “deleted spelling sentinels” also apply: adding back any of the 22 deleted spellings reports an error**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | `type-tables --fix` performs bidirectional completion                                                                                                                                                                                                                                                                                                                                     |
| ~~**T3 Built-in type names consistent with RFC-011 documentation**~~ | Originally: the canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                                                                                                                   | —                                                         | **Deleted following the user ruling of 2026-10-30 (the comparison target does not exist)**: RFC-011 is a generic type-system design and its main text has no table with “built-in type names” as a column                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | — (not applicable; see the explanation after the table for the deletion rationale and coverage)                                                                                                                                                                                                                                                                                           |
| **T4 Exhaustive `Type → MonoType`**                                  | `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                                                                                                                                                         | Two lists                                                 | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Add an arm / remove an arm                                                                                                                                                                                                                                                                                                                                                                |

> **Deletion of T3 and T2’s coverage scope (user ruling, 2026-10-30)**: T3’s **comparison target
> does not exist**—RFC-011 is a generic type-system design, and its main text has no table whose
> column is “built-in type names”; the first implementation had to treat “every inline-code token in
> every Markdown table row that is overall an initial-capital identifier” as the documentation-side
> criterion, which necessarily produced a shell difference requiring no disposition (“nothing to
> do”: 16 registrations, comprising 3 documentation-only entries and 13 code-only entries, none of
> which could be classified as true or false). Therefore, the check, its 16 registrations, and the
> corresponding `--report` / `--json` output sections were **all deleted in the same batch**, with
> no placeholder left in the code.
>
> **The only carriers of built-in type names are the three code-side lists, all of which are now
> reconciled name by name by T2**: ① `from_builtin_name` (`src/frontend/core/types/mono.rs`;
> acceptance surface = 15 canonical names, no aliases—the temporary `DateTime` compatibility alias
> was moved out by S11.1 (issue #513), and `--report` T2.1 measured “14 arms / 15 spellings,” with
> the first spelling of each arm being canonical; the 22 spellings deleted from the implementation
> by 6.1.2 and the `DateTime` moved out by S11.1 are guarded by the 23 T2.5 sentinels); ②
> `CONST_PARAM_TYPES` (`src/frontend/core/sema/probe.rs:57`—moved to L3 by WBS 6.6.9; **a semantic
> subset of ①**, check C1); ③ the LSP built-in type-name list (`src/lsp/world.rs`; **a semantic
> superset of ①**, checks C2 / C3). There is no corresponding documentation-side list: the
> authoritative landing point for a new built-in name is ①, and T2 reports gaps in ②③ as
> **unregistered drift**.

**Implementation notes for T4**: Rust’s exhaustive match already guarantees the “variant → arm”
direction at compile time. T4’s value lies in the **reverse** direction—detecting stale arms in
`lower.rs` that refer to deleted variants (such code would fail to compile when the variant was
deleted, making T4 effectively redundant). **T4 is therefore downgraded to a CI assertion
(`cargo build` succeeding is sufficient) and is not placed on `build.rs`’s `panic!` path.** T1/T2
are the two checks that genuinely require text parsing (T3 was deleted, as noted above).

**T1’s failure form (6.6.2, user ruling 2A, 2026-10-30)**: T1 has changed from “report only” to a
**hard build failure**—`build.rs` calls `type_tables::build_gate(root)`: `Ok(summary)` prints one
`cargo:warning` line (an informational line, not a violation), while `Err(details)` directly
executes `panic!` to reject the build. **All six violation categories use the same failure path**
(see ①–⑤ and ⑦ above; ⑥ belongs to the scan-layer failure path “reconstruction-location registry
disagrees with the source,” and, like the 6.6.3 “different-caliber-generation” case, fails directly,
but is not counted as a gate-violation category). Each error includes the **variant name** and the
construction site / fallback site / variant definition line’s `file:line`—baseline-drift errors do
as well (since 6.6.3 review F1; previously only the numbers were present, leaving readers unable to
determine where to look). Failure to complete the scan itself (file-parse failure or incomplete
caliber) also counts as failure and **must not be downgraded to “passing.”** The registration
mechanism is retained and both tables are now empty: `REGISTERED_ZERO_FORWARD` and
`DEAD_VARIANTS_03` are empty tables (after 6.3 deleted the 13 dead variants, there are no
zero-construction variants). T1.7’s “registration no longer valid / exemption did not match” alarms
and injection-based verification (`*_with(registry)`) continue to work—any newly introduced
zero-construction variant must be deleted or have its rationale registered, or the build fails.

**Single source of truth for reconstruction locations (6.6.2, user ruling 1A)**: classification into
the second category / reverse bridge is driven by the `REBUILD_FUNCTIONS` table in `scan.rs`
(**file + function name → category**). `Type::without_fn_param_names` (the fidelity-preserving
reconstruction introduced in batch 6.5: it recreates an existing AST type to strip type-position
parameter names, with its sole consumption point being normalization before entry into the IR
through `ir_gen.rs`; **removed in 6.6.12 when the IR carrier was narrowed**, with its registration
and caliber snapshot removed in the same batch) belongs to the **second category, AST→AST
reconstruction**. Previously it was counted as parser-forward, artificially adding one forward
construction site to 11 variants (`Name` / `ConstExpr` use `self.clone()` and therefore never
produce a site). The registry is reconciled line by line with the source: if a registered function
has no same-named `fn` in the source, the scan reports an error directly and **must not silently
fall back to the forward category** (otherwise a renamed or deleted reconstruction site would
masquerade as a construction site).

**The baseline pins down two things (6.6.3 review F1/F2)**:

1. **The source tree**—`SCOUT_BASELINE_REV`: only the same source tree can meaningfully have the
   same point counts;
2. **The caliber**—`SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` (= a frozen snapshot of
   `scan.rs::REBUILD_FUNCTIONS` at that revision, recorded by “file + function → category key”). The
   snapshot deliberately does **not** share a table with the registry: sharing one table would cause
   both to change together, making caliber changes undetectable.

The gate performs machine-level reconciliation between the snapshot and the current registry
(`baseline::caliber_drift`, modeled after `scan::missing_rebuild_functions`): adding or removing a
reconstruction function, or changing a function’s category, reports **“caliber and baseline are from
different generations”** (including the caliber revision and each difference), and **skips**
baseline-count comparison—point counts from different calibers are not comparable, and continuing to
compare them would disguise a caliber change as numerical drift. The T1.6 section of the `--report`
output prints the baseline revision and **caliber revision**, allowing anyone reading the report or
running the script to recompute the result using those two revisions.

**T1’s two implementation difficulties and mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and neither does `tools/code-tables`. Mitigation:
   - **Preferred**: introduce `syn` into `tools/type-tables` (only that crate; the root crate
     remains unchanged). `syn` is standard practice, and the gate crate is independent of the
     compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` line-level heuristic (an existing
     Python tool in the repository)—construction sites are on the right-hand side of `=` or in
     function-argument position, while match arms are on the left-hand side of `=>`; approximate the
     distinction using the number of `=>` occurrences.
   - **Conservative starting point (followed in this order to completion)**: when T1 first launched,
     it **only reported and did not fail** (`cargo:warning`), taking one full baseline on the
     **unmodified** code to confirm the heuristic before changing it to `panic!`—since 6.0.1 it only
     reports, and **6.6.2 (2026-10-30) changed it to `panic!`**; by the time it became hard, the
     baseline had already been refreshed under the same caliber to a consistent 13/13 rows. This
     matches the handling principle that “a new CI gate may initially be red and must first
     establish a baseline.”
2. **Reverse bridges and AST→AST reconstruction sites must be classified separately.** Section 2.2
   records the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; section 2.2.2 records the two AST→AST substitution functions in
   `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites in the 13-item list). If
   T1 mixes them with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` / `String` /
   `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` will be classified as having
   construction sites and exempted from deletion, directly contradicting the disposition table in
   5.2. T1’s report must separate the three construction-site categories, and **fallback exemptions
   must apply only to individually registered entries** (currently only the parser error fallback
   for `Void` at `types.rs:836`), never to all wildcard arms.

**Gate coverage (implemented, 6.6.2, user ruling 2A, 2026-10-30)**: `build.rs` calls
`type_tables::build_gate(root)`, immediately after the `code-tables` validation block, following the
existing `panic!`-based build-rejection pattern—`Ok(summary)` prints one `cargo:warning`, while
`Err(details)` executes `panic!("Type-table gate T1 failed (build rejected): …")`. **All six
violation categories use the same failure path**: an unregistered zero-construction variant / a
registered variant that has acquired a forward construction site / a registered exemption site that
no longer matches / an unregistered wildcard-arm fallback site / **caliber and baseline from
different generations** / **survey baseline (T1.6) drift**. Each error includes the variant name and
the construction site, fallback site, or variant definition line’s `file:line` (drift entries have
included construction-site locations since 6.6.3). Failure to complete the scan itself (such as a
file-parse failure) also counts as failure and **must not be downgraded to “passing.”**

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and never touches `ast::Type` variants                                                                                                                                                                                                                                                                                                                                       |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` in `types/substitute.rs:112`, and `substitute` in `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                                                        |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `check_structural_termination` at `478` in `eval/dependent_types.rs`, and `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                                                                                                              |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. The 8 match sites for `AstType::NamedStruct` / `AstType::AssocType` at `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623` were updated. The disposition of `AssocType` was changed to deletion, and all 8 sites were synchronized. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3, now using `canonical_name` + structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Completed (WBS 6.5.2, 2026-10-30)**. The `named_struct` match converged with 6.3; the previous `let params: Vec<MonoType>`, which directly read `signature_params`, now obtains flattened parameters through `Type::fn_params_flat` (the verification conclusion in 5.6: an unfiltered list, from the same source as the old field)                                                                                                |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs modification**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and the assembly at `632`                                                                                                                                                                                                                                                                                            |
| Interpreter (`backends/interpreter/`)                        | **Needs modification**. `image.rs:44` changes `type_table: Vec<ir::Type>` to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects the `{:?}` output format                                                                                                                                                                                                          |
| RFC-027 compile-time types                                   | **Needs modification**. `Type::ConstExpr` is retained but marked **compile-time-only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                                                                                                                                                      |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by the first-layer “type consistency” invariant of 07. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are expression-side forms of the same problem—they may occur inside
`ConstExpr` and therefore likewise must not descend into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (the sole
   forward site for `Void` is an error fallback—**WBS 6.3 changed that fallback to
   `Type::Name{"()"}` and removed the gate-exemption registration**, so there are now no forward
   construction sites) → the parser does not emit them (the disposition of reverse bridges and
   AST→AST reconstruction is covered by #28 / #47 in 6.3 and the 10 sites in 6.3.2, all
   implemented);
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the set of
   types, only the **carrier**;
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction in which behavior can change, and it is a fix rather than
   a regression**, but the third-layer corpus differential in 07 must confirm that no regression
   occurs.

**Semantic change points requiring particular attention**:

| Location                | Current state                                                                                 | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match is sufficient) |

> **Design judgment**: these 8 variants currently all collapse to `Void`, meaning that **every code
> path depending on signature bytecode (the interpreter’s arity/type checks, serialization format)
> currently sees `Void`**. After convergence, these locations will see the real types. **This is the
> only possible source of behavioral change in this document, and its direction is “restoring
> correct information.”** If the corpus differential reveals a difference, it should be treated as
> **exposing an existing defect**, not as a regression introduced by this document. To determine
> this, construct a `.yx` program containing a `Ref` parameter and compare `dump_bytecode` before
> and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                             | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Category                          |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`           | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**completed: WBS 6.3.1, 2026-10-29**—all 13 variants deleted and `ast::Type` reduced to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                        | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43` (original variant location, **deleted**) | Delete `Expr::FnDef` (**completed: WBS 6.5.1, 2026-10-30**—the variant definition and all 15 production consuming arms removed in the same batch, with zero references within `src/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                        | Delete `Assign.signature_params` (**completed: WBS 6.5.2, 2026-10-30**—the same-named field was deleted from `TypeDefinition` in the same batch, which now carries `type_annotation`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026` (original arm location, **deleted**)      | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Follows #3 (completed)            |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                        | Change the `NamedParen` comment to “syntax supplies the binder name; semantic interpretation occurs in the **type-checking layer**” (**completed: WBS 6.5.2, 2026-10-30**—the comment now says “parameter names belong to `Type::Fn.params`; this node carries only the return-position binder”; the initial draft’s `sema/` is D1’s target directory name, whereas the current repository uses `src/frontend/core/typecheck/`)                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                                  | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Addition                          |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                              | Add `trait TypeEnvProbe` + `NullProbe` (**completed: WBS 6.4.1, 2026-10-29**—the landed state also includes `BuiltinProbe` and `NameKind`; the trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Addition                          |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                          | Add the `probe` field and accessors (**completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites are wired**: 4 production + 36 test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                        | Use `state.probe()` to determine `NameKind` (**not done**: 6.4 introduced only the **classification enum + probe queries** and did not change the structure of `Type::Name`—fielding it would affect every production and test construction site and require the reverse bridge `mono_to_ast_type` to faithfully write back `kind`, forcing regeneration of IR snapshots and corpus baselines, which conflicts with this batch’s zero-drift C3 scope; fielding `NameKind` is deferred until a second consumer appears)                                                                                                                                                                                                                                                                                                                                                                    | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                        | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; only add an explanation of the variant deletion)—**not done**: 6.4 did not modify `types.rs` (the change surface in that file belongs to the 6.3 fallback refactoring), and this line is deferred together with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                            | Delete the `_ => (Vec::new(), Type::Void)` fallback and return an explicit error instead—**WBS 6.3.1 changed the fallback to a let-else that explicitly returns `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical), but **the “return an explicit error” half remains undone**—the original plan deferred it to 6.4’s error-path refactoring, but **6.4 did not take over that path** (6.4 only injected type-name dependencies and did not modify `types.rs`) ⇒ **6.5 did not take it over either** (that batch only reshaped representations and left the fallback path unchanged) ⇒ **it moved to 6.6**, where it is registered in the 6.6.1 row of 09. The path is syntactically reachable (the `Name` arm of `semantic_tokens` records one extra dummy-span token; the corpus has 0 matches, so there is no drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                        | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n)` (**completed: WBS 6.4.3, 2026-10-29**—the landed state uses `ParserState::kind_of` to read `NameKind::Predicate`: predicates declared during the current pass take priority, and the rest are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                        | Delete the `typecheck::operator_interfaces::spec()` call (**completed: WBS 6.4.3, 2026-10-29**—changed to `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                            | Delete the `signature_params` argument—**completed (WBS 6.5.2, 2026-10-30)**: all construction sites removed the field; in the same batch, the HashMap name table in the name-merging section was changed to positional alignment (criterion: preceding validation guarantees identical names at each position)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                             | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3 (completed)            |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                            | Retain (the parameter-list retrieval point)—**“change to read Lambda directly as described in 5.5” was not done**: this item was not in the 6.5.1 change list in the first place, and its disposition is to be decided together with the declaration-driven checking mode in 6.8.2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Retain                            |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`           | Delete the 13 match arms (one for each deleted variant) (**completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                        | Prune synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Convergence                       |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                            | Retain the `DateTime` alias (**completed: S11.1 (issue #513)—the alias was moved out; the bare name reports E1003/E1002**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                              | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Addition                          |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                                    | Register `i64` / `int`, etc., as equivalent spellings of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Convergence                       |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`)        | Delete `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                            | Keep `FunctionCode.params` as `Vec<MonoType>` (already so)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                                     | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Convergence                       |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                            | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Convergence                       |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                           | Change the `type_table` conversion to `.collect()` (the elements are now the same type)—the actual change was to **delete the conversion and copy the values unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)               | **Delete the entire `From<MonoType> for IrType` impl** (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                             | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Convergence                       |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                            | Handle the `AstType::AssocType` arms according to the gate conclusion—the **conclusion was reached and implemented: WBS 6.3.2 removed all 5 variants / 10 sites registered in 03 §2.2.2** (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each), and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`                     | Type-table gates T1/T2 (the original T3 was deleted following the user ruling of 2026-10-30)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                            | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                           | Same; obtain the name from `Assign.target` instead                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                                   | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                             | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                           | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                                    | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                        | Delete the `Expr::FnDef` test (**completed: WBS 6.5.1, 2026-10-30**—the case was not deleted; it was rewritten as two Lambda-destructuring cases plus one end-to-end assertion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                        | Rewrite as Lambda + Assign (**completed: WBS 6.5.1, 2026-10-30**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`                  | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                        | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **Must be handled in the same batch as #1**—**the criterion was corrected (completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`); after 6.1.2 removed lowercase spellings, it would become `TypeRef` and **silently lose type identity**. The actual implementation therefore uses the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) plus structural write-back (`Generic` / `Ref` / `Fn`)                                                                                                                                                                                | Convergence                       |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                                     | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)` instead. Callers, including `parser/tests/ast.rs`, pass `&NullProbe` (**completed: WBS 6.4.3, 2026-10-29**—the landed signature is at `ast.rs:927`; the 3 production callers in `ir_gen.rs` pass `SemanticProbe`, and the others pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                          | Delete this file’s `name_used_as_type` and call the consolidated version in `ast.rs`; change the `is_predicate` short circuit to read `NameKind::Predicate` (**completed: WBS 6.4.4, 2026-10-29**—the consolidated implementation is at `ast.rs:848` and covers top-level determination + predicate short circuit + Struct/NamedStruct recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                            | Connect the `CONST_PARAM_TYPES` determination through `NameKind::Builtin` (see 5.3) (**completed: WBS 6.4.5, 2026-10-29**—dual-knowledge connection: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |                                   | !knows_builtin_types())`; a probe that does not hold the built-in name table falls back to the single-table criterion used before 6.4) | Convergence |

> **The 6 landed items of 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed 2026-10-28)
> implemented these 6 items in one batch—four carriers changed to carry `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy `From<MonoType> for IrType` bridge
> and the `ir::Type` re-export alias were deleted, and decoding copies values unchanged
> (`bytecode.rs:2315`, `:2316`, `:2346`). **Their line numbers have been refreshed to the landed
> state** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390` were pre-change
> positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured results and gate results.

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, needs a `mod`
> declaration wired into the build (a lesson from the repository’s 8 orphaned test trees). When
> creating the new crate, register it in the root `Cargo.toml` in the same batch.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/`’s 293 single-file `.yx` corpus files + the multi-file corpus layer
established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library **must
all retain identical behavior** (identical diagnostic codes and messages, with order canonicalized).

| Risk                                                            | Assessment                                                                                                                                                                                                                                                                                                                      | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code uses type annotations such as `i64` / `int` / `f64` | After canonicalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                            | **Measured** (end of 5.3): 0 non-canonical names in actual type positions → 6.1.2 directly deleted the 22 spellings and **did not create a lexer alias table**; the corpus diagnostic set is unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Source code uses `DateTime`                                     | The temporary compatibility alias was moved out of the built-in name table by S11.1 (issue #513)—the alias arm was deleted, with parsing behavior starting at `mono.rs:1123`, using 15 names without aliases—so it is not a built-in name                                                                                       | The compatibility window is closed: before the move, `DateTime` was synonymous with `Int`, so behavior did not change; afterward, a bare `DateTime` annotation becomes `TypeRef` and reports E1003/E1002 (a category-D semantic change, approved by the user)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `("()")` occurs in type position                                | It is not a product of the `Name` form, but the type spelling of `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                                   | **Retain**: `()` and `Void` share an arm, making it one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `dump_bytecode` output changes                                  | The `type_table` element type changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                                        | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only in stages C1/C2). The output diff must still be recorded in stage 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Behavior changes after the lossy bridge is fixed                | See 6.2                                                                                                                                                                                                                                                                                                                         | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `mono_to_ast_type` rewrite changes type-substitution results    | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting the variants, taking the `_` arm changes the substitution result from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Stage 3 targeted comparison: select corpus files with generic substitution and compare the monomorphized products’ `type_name()` before and after the change. **Do not assume that `{:?}` output is equivalent.** **The criterion was corrected (stage 3 measurement)**: equality must not use `type_name()`—it is the lowercase internal name used for diagnostic display, and comparing only `type_name()` can show that both are “`int64`” but cannot distinguish whether the canonical name is `Int64` or `i64`; instead, use **canonical-name round-tripping** (`from_builtin_name` ↔ `MonoType::canonical_name()` for each name) plus a structural comparison of the written-back product. WBS 6.3 measurement: the dumps and `.42` product md5 values of 4 generic-substitution programs were byte-for-byte identical to those before the change |
| `.yx` source code itself                                        | **Zero modifications**                                                                                                                                                                                                                                                                                                          | This document does not require changes to any corpus or standard-library source file. This is a hard acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

**Hard acceptance condition** (**criterion revised on 2026-10-27, following the 2026-10-26 ruling
②**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly driven
test files** under `tests/` may be modified when mechanically required by the change, with every
**file / line count / rationale** registered individually; **tests must not be deleted, and
assertions must not be weakened**. The original criterion (`git diff --stat tests/ src/std/`)
applied a blanket zero-difference requirement to all of `tests/`; this revision relaxes the
requirement by one level for directly driven test files only, while the zero-difference requirements
for the corpus layer (`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library
(`src/std/`) remain unchanged. **Exception**: the 6.8.2 behavior-fix batch is handled in a separate
round with a separate ruling.

## Implementation Highlights

The criterion is uniformly **C3 from 07: identical diagnostic codes and messages (order may be
canonicalized)**. The following is the stage breakdown for this workstream; see RFC-039 for the
global cross-document ordering.

| Stage                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Rollback point                                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2 (T3 landed in stage 0 and was later deleted following the user ruling of 2026-10-30). The first version of T1 **only reports and does not fail**. Take a full baseline on the **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | The T1 report accurately lists the 13 **parser-forward zero-construction variants** (including the classification of `AssocType`: 0 forward / 2 AST→AST, with D8 unchanged), and **separately lists the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**; it reports the 13 parser-forward zero-construction variants separately from the 1 globally zero-construction variant (`Enum` only); T2 is fully green or provides a remediable list | No code changes; no rollback required                                                                                                                                                                                                                                                                                                                                                                    |
| **1. Name canonicalization**                  | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) converged to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; `add_builtin_types`’ 7 lowercase copies, the LSP list’s `Uint` / `Int128`, and `sizeof`’s `Uint` arm were cleaned up in the same batch; **no lexer alias table was created** (the corpus measurement found 0 non-canonical names). **Verify in parallel whether `classify_generic_params` uses `signature_params` or `Lambda.params`** (**verified: it uses the unfiltered original signature list; after 6.5.2 it is provided by `Type::fn_params_flat`, see 5.6**)                                                                                                                                                                                                                                                                | C3: the diagnostic set for the entire corpus is **identical entry by entry** to the stage 0 baseline                                                                                                                                                                                                                                                                                                                                                                                        | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                              |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: four carriers changed to carry `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy `From<MonoType> for IrType` bridge (formerly `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias were deleted (the `pub use` line at the head of `ir.rs` was reduced to a private `use`; that private `use` was later deleted by WBS 6.6.12); decoding now copies values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), so the previous silent information loss in which 8 `MonoType` categories collapsed to `Void` no longer occurs                                                                                                                                                                                                                   | C3 + targeted `dump_bytecode` comparison (each row of the 6.2 table)—**measured fully green** (recorded in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                             | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                              |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: deleted 13 variants according to the table in 5.2 (`ast::Type` 26 → 13); removed match arms such as `mono.rs:699-708` and changed the fallback at `types.rs:836` (to a let-else that explicitly returns `Type::Name{"()"}`); **handled `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; updated `types/tests/mono.rs`. **The criterion for #47 was corrected**: the original “uniformly use `AstType::Name` + `mono.type_name()`” became invalid after 6.1—`type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`), and after 6.1.2 removed lowercase spellings it would become `TypeRef` and **silently lose type identity**. The actual implementation used the new `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) plus structural write-back | C3 + a fully green `cargo build` (exhaustive matches force every arm to be completed) + targeted comparison of generic substitution (last row of 6.4)—**measured fully green** (recorded in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                            | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                              |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: added `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); changed `parser_state.rs` (adding the `probe` field and the `probe()` / `kind_of()` accessors, with **all 40 construction sites wired** = 4 production + 36 test); deleted the 2 parser-side `operator_interfaces::spec()` calls and changed the 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); consolidated the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connected `CONST_PARAM_TYPES` through the dual knowledge provided by `NameKind::Builtin` (#50). **Not done**: adding a `kind` field to `Type::Name` at `types.rs:162-165` (#10; fielding it conflicts with this batch’s C3 scope), and changing the fallback at `types.rs:836` to return an explicit error (#12, moved to 6.5)      | C3 + **new assertions**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps measured (`typecheck` / `operator_interfaces`) returned 0 matches**; corpus 356/356, 216 IR snapshots with zero drift, and `cargo test --all` exited 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                        | Separate commit; revertible                                                                                                                                                                                                                                                                                                                                                                              |
| **5. Dead AST variants and dedicated fields** | **Completed (WBS 6.5.1 / 6.5.2 + merged 6.8.1, 2026-10-30)**: deleted `Expr::FnDef` (baseline measurement: **25 sites / 16 files** = 1 definition + 15 production consumption sites + 3 test construction sites + 6 comments, of which 19 were code references) and deleted the parallel parameter tables from `Assign` / `TypeDefinition`; reshaped `Type::Fn.params` into `Vec<Param>` (names and types co-located) → a single flattening entry point, `Type::fn_params_flat`; retained `NamedParen` as a syntax node, with the binder explicitly interpreted by the type checker                                                                                                                                                                                                                                                                                                                                 | C3 + all **22 modules** in `tests/integration/` green (measured 22 `.rs` files; the original text said 18, an old count); parameter-position formatter output remains byte-for-byte identical (all 355 `.yx` corpus files compared byte for byte); corpus 356/356, 216 IR snapshots with zero drift; `#[test]` count within `src/` 2878 → 2883 (no tests deleted)                                                                                                                           | Seven commits (2 deleting the variant + 3 reshaping representations and deleting the old representation + 2 fix-up commits); **revert policy**: the commit deleting the variant can be reverted independently, while the test-migration commit must be reverted together with it (see “Registration ①” in [09a](./09a-wbs-history.md) §P6 6.5); see §P6 6.5 in 09a for measured results and gate results |
| **6. Gates become hard failures**             | **Completed (6.6.2, user ruling 2A, 2026-10-30)**: T1 changed from warning to `panic!`, and **T1.6 survey-baseline reconciliation was connected to the same failure path** (a mismatch between construction-site counts and `SCOUT_BASELINE` fails the build); under ruling 1A, `without_fn_param_names` was moved into the second category, returning the forward count to its frozen value; T4 was downgraded to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | `cargo build` **must fail** when a zero-construction variant is deliberately introduced; changing any `SCOUT_BASELINE` variant count by 1 **must also fail**—both mutations were measured using mechanically reversible patches (`scripts/dev/scratch.py trial`)                                                                                                                                                                                                                            | Separate commit                                                                                                                                                                                                                                                                                                                                                                                          |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage 2**—before stage
2 lands, the bytecode-side reverse bridge A (`bytecode.rs`’s `From<MonoType> for IrType`) consumes
`IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation. That
bridge was deleted in 6.2.2, meaning that **the remaining rewrite surface when stage 3 deletes the
variants is reverse bridge B** (`middle/passes/mono/function.rs`’s `mono_to_ast_type`, which still
resolves `String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`.
Stage 3’s #47 must be handled in the same batch as #1; otherwise, `mono_to_ast_type` silently
changes behavior. **Stage 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B was changed to
use `canonical_name` + structural write-back; **the three `ast::Type` carriers in
`middle/core/ir.rs` were not changed**—6.3 only deleted variants and did not alter the IR shape (216
IR snapshots and 356 corpus baselines showed zero drift), and carrier disposition **was actually
completed in 6.6.11 / 6.6.12** (deleting two zero-producer dead variants and replacing `TypeDecl`
with an IR-native narrowed carrier; `ir.rs` no longer imports `ast`). The original plan to “leave it
for stage 4” was not fulfilled; this paragraph takes precedence.

**Dependency on downstream work**: only after all stages in this document are complete may
[04-ssa.md](04-ssa.md) begin. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all have the same element type, giving SSA type annotations a sole landing point.

### Mandatory Validation for Each Stage

| Stage | Must execute                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, then manually verify the T1 report (**including the separately reported reverse-bridge reconstruction sites**)                                                                                                                                                                                                                                                                                                       |
| 1-5   | `cargo test`; full-corpus differential for `tests/yaoxiang/` (07’s third layer); entry-by-entry comparison under C3                                                                                                                                                                                                                                                                                                                                                                        |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                         |
| 3     | Additionally: for corpus files containing generic substitution, compare the monomorphized products’ `type_name()` before and after the change—**the criterion was corrected (stage 3 measurement)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient to establish equality; use **canonical-name round-tripping** (`from_builtin_name` ↔ `MonoType::canonical_name()` for each name) plus a structural comparison of the written-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                            |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                                                                                                                                       | Rationale                                                                                                                                                                                                                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                                                                                                                                        | The `ir::Type` alias (the `pub use` line at the head of `ir.rs` before 6.2.2) directly caused the field-type mismatch between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After the alias is deleted, the two field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                                                                                                                                      | It would introduce an L2→L3 reverse dependency (violating RFC-039 layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See the complete argument in 5.1                                                                                                                             |
| Do not delete `NamedParen`   | Retain it as a pure syntax node; move binder semantics into the **type-checking layer** (`src/frontend/core/typecheck/checker/refinement.rs:421`; `sema/` is D1’s target directory name, while the current repository directory is `typecheck/`) | `ast.rs:518-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as the binder                                                                                                                                 |
| Directory rename             | **Do it** (D1), as a pure move batch at the end of P5/P6; paths in this document use the pre-rename current state                                                                                                                                | Ruling of 2026-10-03 (ChenXu233); supplementary construction method on 2026-10-04                                                                                                                                                                                                                    |

### Rejected Alternatives

None of the four alternatives was adopted: **retaining all three representations and merely adding a
conversion layer** resolves none of the root causes, while `ir::Type` remains non-independent and
neither the dead variants, the synonym table, nor the reverse dependency disappears; **using a macro
to generate the synonym table** treats the symptom rather than the cause—after `Type::Int(usize)` is
deleted, the macro has no reason to exist, and consistency should instead be guaranteed by the T2
gate, whose verifiability is stronger than that of a macro expansion; **deleting only the dead
variants without unifying the representation** has the lowest cost but leaves the problem in place,
with no gate to prevent the next person from adding another `Type::Int128(usize)` and recreating the
same dead-variant pattern (if only one commit can be delivered, this approach should explicitly be
recorded as a subset of that alternative, and the T1 gate must be delivered alongside it);
**unifying on `MonoType`** is addressed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 19 locations**. `Expr::FnDef`’s 15 production consumption
  sites (including 3 exhaustive-match arms) + 3 test construction sites + 1 variant definition form
  a parallel path that never executes; deleting it removes one branch each from `ir_gen.rs`,
  `ownership.rs`, and `termination.rs`.
- **Eliminates one already-diverged duplicate implementation**. The two copies of
  `name_used_as_type` (2.4) already differ in definition-body field recursion; after consolidation,
  only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value.”** This is
  the prerequisite for the first-layer “type consistency” invariant of 07 to pass.
- **The gates make regression impossible**. T1 turns “adding a zero-construction variant” from a
  silent pass into a build failure—the repository’s existing `build.rs` mechanism demonstrates that
  this path is viable.
- **Zero source-code changes**. None of the 293 `.yx` corpus files or the standard library requires
  modification, which is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may change behavior**. After the lossy bridge is fixed, 8 `MonoType` variants become
  visible as their real types rather than `Void`. The direction is a fix, but it changes paths that
  depend on signature bytecode. This is the largest uncertainty in this document.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify. This is why stage 0 requires manual
  verification.
- **If T1 misses reverse bridges, the disposition table and gate contradict each other**. Section
  2.2 proves that 7 “dead variants” have production reconstruction sites in reverse bridges. If T1
  does not separate construction-site directions, those 7 will be classified as having construction
  sites and exempted from deletion, making the gate an obstacle to the deletion work rather than a
  safeguard. | **2. Deleting `signature_params` carries semantic risk** (**verified and resolved:
  6.5.2, 2026-10-30**). It carries the **unfiltered** parameter list, whereas `Lambda.params` is
  **filtered**. If `classify_generic_params` depends on the former, the refactoring would change
  generic-classification results. **Measured conclusion**: `classify_generic_params` /
  `extract_generic_param_names` require the unfiltered list; flattening `Type::Fn.params` and the
  old `signature_params` are from the same source (the same `Param` nodes), so the classification
  result does not change. The “use `Lambda.params`” approach would change behavior and **was not
  adopted** (corpus 356/356, 216 snapshots, fully green `cargo test --all`). | | **3. Stage 3
  affects `middle/passes/mono/function.rs`**. In addition to the 8 `AssocType` match sites,
  `507-529`’s `mono_to_ast_type` is the exit point for monomorphized type substitution, and
  rewriting it may change the substitution result’s `type_name()` (last row of 6.4). |
- **Non-canonical type names in the `.yx` corpus may be nonzero**. If the corpus extensively uses
  `i64` / `int`, stage 1’s “directly delete synonyms” would be infeasible and would have to be
  replaced with “move them into a lexer alias table,” reducing the convergence achieved by
  `from_builtin_name`. (**Measured as 0**: the 6.0 survey in the end of 5.3 led WBS 6.1.2 to delete
  the aliases directly rather than use an alias table.)
- **The gate may initially be red** (**this risk was addressed in two steps**). If T1 launched
  directly with `panic!`, it would immediately block compilation; therefore, since 6.0.1 it first
  only reported to establish a baseline, and only in 6.6.2 did it change to `panic!`. By the time it
  became hard, the baseline had already been refreshed under the same caliber to a consistent 13/13
  rows, and the current tree has 0 zero-construction variants.

### Open Questions

> **All open questions formerly listed in this section have been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document leaves no pending items.**

## See Also

### Overarching and Peer Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Document)](../../rfc/accepted/039-compiler-architecture.md)
  — overarching master document; four-layer model, criterion tiers, and routing table C (the 3
  type-representation-related entries are assigned to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction
  specifications, and target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document’s prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; convergence of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, and the first-layer IR validator’s type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — generic mechanism
  and type-level constructors (`If` / `Match`); **does not contain a built-in type-name list**, so
  it is not a gate comparison target (the original T3 was deleted following the user ruling of
  2026-10-30; see 5.7)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of a build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

> **Line anchors were refreshed to the landed state of 6.5** (2026-10-30); entries deleted by
> earlier batches are retained and marked with their deletion batch.

- `src/frontend/core/parser/ast.rs:441-541` — `ast::Type` definition (**13 variants** after 6.3
  closure; the 26-variant era spanned 427-541)
- `src/frontend/core/parser/ast.rs:518-532` — explanation of `Type::NamedParen`’s binder semantics
  (evidence that deleting it would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:778`, `:985` — `Param` (names and types co-located) and
  `Type::fn_params_flat` (flattening entry point): the sole parameter representation after
  6.5.2/6.8.1
- (**Deleted, WBS 6.6.12**) `Type::without_fn_param_names` in `src/frontend/core/parser/ast.rs`: a
  print-boundary closure introduced in batch 6.5 for “IR carrying `ast::Type`”; after 6.6.12
  replaced `FunctionBody::TypeDecl` with an IR-native narrowed carrier, AST no longer enters the IR,
  the function lost its sole production call site, and it was deleted (T1’s `REBUILD_FUNCTIONS` /
  `SCOUT_BASELINE_CALIBER` lost one entry in the same batch)
- `src/frontend/core/typecheck/probe.rs` (from WBS 6.6.9) — `CONST_PARAM_TYPES`: the sole
  authoritative table of **acceptable type names for const generic parameters** (originally in
  `parser/ast.rs` as the third copy of L3 knowledge; moved back to L3 as the parser’s closed set was
  cleared, and the parser now asks through `TypeEnvProbe::is_const_param_type`; see 5.4)
- `src/frontend/core/parser/ast.rs:1097` — `name_used_as_type`: the single consolidated
  implementation after 6.4 (the original `name_used_as_type_in` and the copy in `declarations.rs`
  were merged)
- `src/frontend/core/parser/ast.rs:1178`, `:1275` — `extract_generic_param_names` and
  `classify_generic_params` (classified through the injected probe since 6.4)
- `src/frontend/core/parser/ast.rs:196-218`, `:222-228` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:407-416` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:823` — `extract_fn_type_info`: fallback for non-`Fn`
  forms (returns `Type::Name{"()"}` since 6.3; the `Type::Void` variant was deleted)
- (**Deleted, 6.4**) `name_used_as_type` originally at
  `src/frontend/core/parser/statements/declarations.rs:42-80`, merged into the single implementation
  in `ast.rs`
- `src/frontend/core/parser/statements/declarations.rs:441` — hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call (changed to `ParserState::kind_of` / the injected
  probe in 6.4; both literal references disappeared, leaving only the comment)
- `src/frontend/core/parser/pratt/nud.rs:512` — expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:431` — the `expr_to_params` arm that retrieves the
  parameter list
- `src/frontend/core/types/mono.rs:1123` — the `from_builtin_name` built-in name resolution table
  (15 names, no aliases—`DateTime` was moved out by S11.1 (issue #513); final wording from WBS
  6.1.2)
- `src/frontend/core/types/mono.rs:1216` — the `From<ast::Type> for MonoType` implementation (13
  match arms)
- `src/frontend/core/types/mono.rs:169` — the `MonoType` definition (approximately 22 variants)
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
  `FunctionCode`’s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:458` — `mono_to_ast_type`: the reverse bridge rewritten in 6.3
  (using `canonical_name` + structural write-back)
- `src/backends/interpreter/image.rs:44` — the interpreter module’s `type_table`
- `build.rs:33-51` — build-time error-code registry gate (the model for the type-table gate)
- `build.rs:53-69` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; introducing `syn` into `tools/type-tables` is the first exception)
