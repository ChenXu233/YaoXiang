---
title: Type Trait System Implementation Design (Dup/Clone)
status: draft
created: 2026-05-29
---

# Type Trait System Implementation Design

## Goal

Implement the `Dup` trait (implicit shallow copy marker) in the compiler type system, completing the trait system's recursive checking capability.

## Core Design

### Dup Trait Definition

```rust
// Marker trait at the same level as Clone and Debug
// No methods — type-level marker only
TraitDefinition {
    name: "Dup",
    methods: {},           // empty — marker trait
    parent_traits: vec!["Clone"],  // Dup implies Clone
    generic_params: vec![],
    is_marker: true,
}
```

### Which Types Are Dup

| Type | Dup | Reason |
|------|-----|--------|
| Int, Float(32), Float(64) | ✅ | Primitive |
| Bool, Char | ✅ | Primitive |
| String, Bytes | ✅ | Already internally reference-counted |
| &T (ReadToken) | ✅ | Zero size, compile-time concept |
| &mut T (WriteToken) | ❌ | Linear, exclusive unique |
| struct | auto-derive | All fields Dup → struct Dup |
| Fn (closure) | ❌ | Closure-captured environment may not be Dup |
| Arc(T) | ✅ | Arc itself can shallow-copy |

### Relationship Between Dup and Clone

```
Dup  →  Clone   (all Dup types automatically implement Clone)
Clone  ↛  Dup   (has Clone does not imply has Dup)
```

## Implementation Checklist

### 1. trait_data.rs — Add is_marker Field

**File**: `src/frontend/core/types/base/trait_data.rs`

```rust
pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    pub parent_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub span: Option<Span>,
    pub is_marker: bool,  // NEW: marker trait with no methods
}
```

For traits with `is_marker = true`, no method implementation checking is required. Compiler handling of marker traits:
- Primitive types → auto-register impl
- struct → auto-derive with recursive field checking
- Generic bounds `T: Dup` → handled like ordinary trait bounds

### 2. std_traits.rs — Register Dup, Remove Send/Sync

**File**: `src/frontend/core/typecheck/traits/std_traits.rs`

```rust
// Modify STD_TRAITS (remove Send, Sync, add Dup)
pub const STD_TRAITS: &[&str] = &[
    "Clone",
    "Dup",      // NEW
    "Equal",
    "Debug",
    "Iterator",
];

// New function
fn add_dup_trait(trait_table: &mut TraitTable) {
    trait_table.add_trait(TraitDefinition {
        name: "Dup".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["Clone".to_string()],
        generic_params: vec![],
        span: None,
        is_marker: true,
    });
}

// In init_primitive_impls, register Dup for primitives
// Int, Float, Bool, Char, String, Bytes all automatically get Dup impl
```

### 3. solver.rs — Support Recursive Struct Checking

**File**: `src/frontend/core/typecheck/traits/solver.rs`

Core change: `check_dup_trait` method must recursively descend into struct fields.

```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        // Primitives: auto Dup
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool 
        | MonoType::Char | MonoType::String | MonoType::Bytes => true,
        
        // Arc: auto Dup (reference-counted semantics)
        MonoType::Arc(_) => true,
        
        // Ref (borrow token): &T Dup, &mut T not Dup
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Ref { mutable: true, .. } => false,
        
        // struct: recursively check all fields
        MonoType::Struct(s) => {
            s.fields.iter().all(|(_, field_ty)| self.check_dup_trait(field_ty))
        }
        
        // Tuple: recursively check all elements
        MonoType::Tuple(elems) => {
            elems.iter().all(|t| self.check_dup_trait(t))
        }
        
        // Enum: check all variants' all fields
        MonoType::Enum(e) => {
            e.variants.iter().all(|v| 
                v.fields.iter().all(|(_, t)| self.check_dup_trait(t))
            )
        }
        
        // Everything else: default not Dup
        _ => false,
    }
}
```

The same pattern applies to `check_clone_trait` — previously only recognized primitives, now must recurse into struct.

### 4. auto_derive.rs — Support Complex Types and Recursion

**File**: `src/frontend/core/typecheck/traits/auto_derive.rs`

Critical issue with current `can_auto_derive`: when encountering `List[Int]` as `Type::Generic`, it directly returns false.

```rust
pub fn can_auto_derive(
    trait_table: &TraitTable,
    trait_name: &str,
    fields: &[StructField],
) -> bool {
    for field in fields {
        if !field_type_satisfies(trait_table, trait_name, &field.ty) {
            return false;
        }
    }
    true
}

// NEW: recursively check if field types satisfy trait
fn field_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &Type,
) -> bool {
    match ty {
        // Simple type name → look up trait table
        Type::Name { name, .. } => {
            trait_table.has_impl(trait_name, name)
        }
        
        // Generic type List(Int), Option(Point) → check inner
        Type::Generic { name, args, .. } => {
            // Container itself implements trait and all args do too
            if !trait_table.has_impl(trait_name, name) {
                return false;
            }
            args.iter().all(|arg| field_type_satisfies(trait_table, trait_name, arg))
        }
        
        // Tuple → check all elements
        Type::Tuple(elems) => {
            elems.iter().all(|e| field_type_satisfies(trait_table, trait_name, e))
        }
        
        // Function type → functions cannot Dup (conservative)
        Type::Fn { .. } => false,
        
        // Others not derivable
        _ => false,
    }
}
```

### 5. resolution.rs — Complete Trait Resolution

**File**: `src/frontend/core/typecheck/traits/resolution.rs`

```rust
fn find_trait_definition(&self, name: &str) -> Option<String> {
    match name {
        "Clone" => Some("std::Clone".to_string()),
        "Dup" => Some("std::Dup".to_string()),     // NEW
        "Debug" => Some("std::fmt::Debug".to_string()),
        "Equal" => Some("std::cmp::Equal".to_string()),
        "Iterator" => Some("std::iter::Iterator".to_string()),
        _ => None,
    }
}
```

### 6. bounds.rs — Dup Constraint Support

**File**: `src/frontend/core/typecheck/inference/bounds.rs`

The bounds checker existing code already supports `T: Clone` patterns. Adding `T: Dup` works automatically — it calls `trait_solver.check_trait(ty, "Dup")`.

The only thing to ensure: when `check_trait` fails, for struct types, try auto-derive first.

```rust
pub fn check_trait_bounds(&mut self, ty: &MonoType, bounds: &[String]) -> Result<()> {
    for bound in bounds {
        if !self.trait_solver.check_trait(ty, bound) {
            // Try auto-derive
            if let MonoType::Struct(s) = ty {
                if can_auto_derive_for_monotype(&self.trait_table, bound, s) {
                    continue;  // auto-derive passed
                }
            }
            return Err(TypeError::TraitBoundFailed { ... });
        }
    }
    Ok(())
}
```

### 7. mono.rs — No Changes Needed for MonoType (Currently)

`MonoType` doesn't need a `TypeFlags` addition. Dup determination is entirely through the trait system — just query `trait_table.has_impl("Dup", type_name)`. This is a type-checking-time operation, not a hot path.

If performance is needed in the future, a `Cache<TypeId, bool>` could cache lookup results. Not needed now.

### 8. Clean Up Send/Sync

**File**: `src/frontend/core/typecheck/traits/std_traits.rs`
- Remove "Send", "Sync" from `STD_TRAITS`
- Delete `add_send_trait()`, `add_sync_trait()`

**File**: `src/middle/passes/lifetime/send_sync.rs`
- Delete entire checker, or keep as no-op (conservative)
- Remove `send_sync_checker` field from `OwnershipChecker`
- Remove `SendSyncChecker` import and call from `mod.rs`

**File**: `src/middle/passes/lifetime/error.rs`
- Delete `OwnershipError::NotSend`, `NotSync` variants (or keep but mark deprecated)

## Implementation Order

1. **trait_data.rs** — Add `is_marker` field (5 lines changed)
2. **std_traits.rs** — Register Dup, delete Send/Sync, register primitive dup impl (~50 lines changed)
3. **solver.rs** — Recursive struct checking (~30 lines changed)
4. **auto_derive.rs** — Support generic parameter checking (~50 lines rewritten)
5. **resolution.rs** — Add Dup path (1 line)
6. **bounds.rs** — auto-derive integration (~10 lines)
7. **Clean up Send/Sync** — Delete related code

Estimated total changes: ~200 lines. Changes concentrated in 6 files in the trait system directory.

## Verification Approach

```yaoxiang
# Test 1: Primitives auto Dup
x: Int = 42
y = x        # ✅ Int: Dup
print(x)     # ✅

# Test 2: struct auto-derive
Point2D: Type = { x: Float, y: Float }
p = Point2D(1.0, 2.0)
q = p         # ✅ Point2D: Dup (both fields are Float: Dup)
print(p)      # ✅

# Test 3: struct with non-Dup field
Buffer: Type = { data: Array(Int), len: Int }
b = Buffer(...)
b2 = b        # ❌ Move (Array not Dup)
print(b)      # ❌ Already moved

# Test 4: Generic constraint
dup_use: (x: T: Dup) -> T = x  # ✅ T: Dup constraint
```

## References

- Gap Analysis Exploration (Type System Integration Gaps)
- Gap Analysis Exploration (Trait System Gaps)
- [RFC-011 Generic Type System Design](../../design/rfc/accepted/011-generic-type-system.md)
- [RFC-009 Ownership Model v9](../../design/rfc/accepted/009-ownership-model.md)