# RFC-010 / RFC-011 Constraint Design Decisions

> **Status**: Established
> **Created**: 2026-02-02
> **Last Updated**: 2026-02-02
> **Related RFC**: [RFC-010 Unified Type Syntax](../design/accepted/010-unified-type-syntax.md), [RFC-011 Generic Type System](../design/accepted/011-generic-type-system.md)

---

## 1. Core Design Principles

### 1.1 Constraint = Capability Requirements

Constraints define capabilities (fields and methods) that a type must provide:

```yaoxiang
# Define constraint (what capabilities are required)
type Drawable = {
    draw: (Self, Surface) -> Void,
    bounding_box: (Self) -> Rect
}

type Serializable = {
    serialize: (Self) -> String
}
```

### 1.2 Constraints Can Only Be Used in Generic Context

**❌ Not allowed**: Direct duck typing assignment

```yaoxiang
let d: Drawable = Circle(1)  # Not allowed!
```

**✅ Allowed**: Generic constraint

```yaoxiang
draw: [T: Drawable](item: T) -> Void = (item) => {
    item.draw(screen)
}

# Automatically checked at call site
draw(Circle(1))  # ✅ Circle has draw, passes
draw(Rect(2))    # ❌ Rect has no draw, compile error
```

**Reason**:
- Direct assignment is "closing the barn door after the horse has bolted", may accidentally match
- Generic constraints are "verification upfront", intent is clear

---

## 2. Usage Scenarios

### 2.1 Generic Function Parameters

```yaoxiang
# Handle any drawable object
process: [T: Drawable](items: List[T]) -> Void = (items) => {
    for item in items {
        item.draw(screen)
    }
}
```

### 2.2 Generic Return Types

```yaoxiang
# Return serializable objects
serialize_all: [T: Serializable](items: List[T]) -> List[String] = {
    items.map((item) => item.serialize())
}
```

### 2.3 Generic Data Containers

```yaoxiang
# Container elements must be drawable - incorrect写法
let shapes: List[Drawable] = []  # Error! Constraints cannot be used in non-generic context

# Correct: Use generic parameter
let shapes: List[Circle] = []
```

---

## 3. Structural Subtyping Rules

### 3.1 Matching Rules

```yaoxiang
# Constraint definition
type Config = {
    load: () -> String,
    save: (String) -> Void,
    name: String
}

# Type definition
type File = {
    filename: String,
    load: () -> String,
    save: (String) -> Void,
    size: Int,          # Extra field, ignored
}

# Check if File satisfies Config:
#   - load: () -> String ✅ Match
#   - save: (String) -> Void ✅ Match
#   - name: String ❌ No match (field name is filename, not name)
#
# Result: ❌ File does not satisfy Config
```

### 3.2 Matching Algorithm

| Requirement | Type Provides | Result |
|-------------|---------------|--------|
| `x: Int` | `x: Int` | ✅ Match |
| `x: Int` | `y: Int` | ❌ No match |
| `x: Int` | `x: String` | ❌ No match |
| `fn: (A) -> B` | `fn: (Self, A) -> B` | ✅ Match (Self removed) |
| Required field/method | Extra fields/methods | ✅ Ignored |

### 3.3 Compiler Check Flow

```rust
fn check_type_satisfies_constraint(
    typ: &MonoType,
    constraint: &MonoType,
) -> Result<(), ConstraintCheckError> {
    // 1. Verify constraint is valid (all fields are function types)
    if !constraint.is_valid_constraint() {
        return Err(ConstraintCheckError::NotAConstraint);
    }

    // 2. Iterate over all requirements of the constraint
    for (name, required_type) in constraint.required_fields() {
        match lookup_type_field(typ, name) {
            Some(found_type) => {
                // 3. Check type compatibility
                if !types_compatible(found_type, required_type, typ) {
                    return Err(ConstraintCheckError::TypeMismatch {
                        field: name,
                        expected: required_type,
                        found: found_type,
                    });
                }
            }
            None => {
                // 4. Missing required field
                return Err(ConstraintCheckError::MissingField {
                    field: name,
                    constraint: constraint.name(),
                });
            }
        }
    }

    Ok(())
}
```

---

## 4. Constraint Declaration in Type Definition (Optional)

Types can declare which constraints they implement when defined, for code readability and IDE hints:

```yaoxiang
# Declare constraint implementation when defining type
type Point = {
    x: Int,
    y: Int,
    draw: (Point, Surface) -> Void,
    bounding_box: (Point) -> Rect,
    serialize: (Point) -> String,
    Drawable,      # Declare implementing Drawable
    Serializable   # Declare implementing Serializable
}
```

**Effect**:
- ✅ Self-documenting code
- ✅ IDE can hint "Point implements Drawable"
- ✅ Compiler verifies declaration is correct

---

## 5. Error Handling

### 5.1 Error Types

```rust
pub enum ConstraintCheckError {
    #[error("'{0}' is not a valid constraint (must have function fields only)")]
    NotAConstraint(String),

    #[error("Type '{type}' does not satisfy constraint '{constraint}': missing field '{field}'")]
    MissingField {
        type_name: String,
        constraint: String,
        field: String,
        span: Span,
    },

    #[error("Type '{type}' does not satisfy constraint '{constraint}': field '{field}' type mismatch")]
    TypeMismatch {
        type_name: String,
        constraint: String,
        field: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Constraint '{0}' can only be used in generic context")]
    NotInGenericContext {
        constraint_name: String,
        span: Span,
    },
}
```

### 5.2 Error Examples

```
Error: Type 'Rect' does not satisfy constraint 'Drawable'
  Required method 'draw: (Rect, Surface) -> Void' not found
  Note: Add 'draw' method to Rect to satisfy Drawable

Error: Constraint 'Serializable' can only be used in generic context
  Did you mean: 'serialize_all: [T: Serializable](List[T]) -> List[String]'
```

---

## 6. Why This Design?

### 6.1 Comparing Alternatives

| Approach | Problem |
|----------|---------|
| `let d: Drawable = Circle(1)` | Accidental match, duck typing too loose |
| `impl Drawable for Circle` | Requires new keyword, violates RFC-010 design principles |
| `as Config` conversion | Increases syntax complexity |
| **Current approach: Generic constraints** | Clear intent, compile-time check, no accidental matches |

### 6.2 Design Principles

1. **Constraint = Capability requirement**: Only defines "what is needed"
2. **Generics = Upfront verification**: Checked before invocation, no accidents allowed
3. **Zero new keywords**: Reuse existing syntax
4. **Compile-time safety**: All checks completed at compile time

---

## 7. File Structure

```
src/frontend/
├── core/
│   └── type_system/
│       └── mono.rs              # MonoType extension (is_constraint)
│
└── typecheck/
    ├── checking/
    │   ├── mod.rs               # Export constraint module
    │   └── constraint.rs        # ⬅️ Constraint checker (new)
    ├── errors.rs                # TypeError extension
    └── ...
        └── tests/
            └── test_constraint.rs # ⬅️ Constraint check tests (new)
```

---

## 8. Acceptance Criteria

- [ ] Constraint definition syntax works correctly
- [ ] Generic constraint `[T: Drawable]` works correctly
- [ ] `let d: Drawable = ...` is correctly rejected
- [ ] Structural matching rules correctly implemented
- [ ] Error messages are clear and accurate
- [ ] All existing tests pass
- [ ] New 10+ unit tests added

---

## 9. Q&A

### Q: Why is `let d: Drawable = Circle(1)` not allowed?

A: This is "verification after the fact". If Circle happens to have a draw method, it's accepted, which may not be the design intent. Generic constraints are "verification upfront", the code explicitly says "I need Drawable capability".

### Q: How to store a collection of Drawable objects?

A: Use generic containers or trait pattern:

```yaoxiang
# Method 1: Unified concrete type
let shapes: List[Circle] = []
shapes.push(Circle(1))

# Method 2: Use trait/interface pattern (requires runtime dispatch)
# If heterogeneous collections are truly needed, trait object support can be added in the future
```

### Q: How is this different from Rust's Trait?

A: Essentially similar, but:
- No `impl` keyword
- No explicit declaration requirement (optional)
- Only used in generic context

### Q: Can constraints contain data fields?

A: Yes. YaoXiang constraints are not limited to methods:

```yaoxiang
type HasPosition = {
    x: Int,
    y: Int
}

move: [T: HasPosition](item: T, dx: Int, dy: Int) -> T = (item, dx, dy) => {
    # item.x and item.y must exist
    item
}
```

---

## 10. Related Documents

- [RFC-010 Unified Type Syntax](../design/accepted/010-unified-type-syntax.md)
- [RFC-011 Generic Type System](../design/accepted/011-generic-type-system.md)