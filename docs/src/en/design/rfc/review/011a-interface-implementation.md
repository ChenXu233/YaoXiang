---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-14'
updated: '2026-07-05'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and supersedes RFC-011 §2.1-2.4 on interface constraints.**

## Abstract

RFC-011 defines the generic type system but does not specify the interface implementation mechanism in detail. This document supplements:

1. **Interface Declaration**: Interface names directly in type definitions, no `impl` keyword needed
2. **Method Implementation**: Both internal and external declarations supported
3. **Overloading Rules**: Different signatures allow overloading, same signature reports error (no overriding)
4. **Default Values**: Direct `= value` after field
5. **Dynamic Dispatch**: Compile-time type collection + interface matching, no vtable

**Core Design**:

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type definition (internal declaration)
Dog: Type = {
    x: Int = 10,
    Animal,  # Interface declaration
    speak: (Self) -> String = "Woof",
}

# External declaration (overloading)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Eliminated Complexity**:

- ❌ No `impl` keyword
- ❌ No `dyn Trait + 'a` annotation
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No overriding (unified overloading rules)

---

## Motivation

### RFC-011's Gaps

RFC-011 defines the generic type system but does not specify:

| Issue           | Description                         |
| --------------- | ----------------------------------- |
| Interface declaration syntax | How to declare a type implements an interface? |
| Method implementation location | Internal or external declaration? |
| Overloading rules | How to handle methods with the same name? |
| Default value syntax | How to set default values for fields? |
| Dynamic dispatch | How to implement heterogeneous containers? |

### Design Goals

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementations supported both internally and externally
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero-overhead**: No vtable, compile-time type collection

### Comparison with Rust

| Feature      | Rust                            | YaoXiang                          |
| ------------ | ------------------------------- | ---------------------------------- |
| Interface declaration | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| Method implementation | In `impl` block             | Internal or external               |
| Overloading  | Not supported                   | Supported (different signatures)   |
| Default values | Requires `#[default]`         | Direct `= value`                   |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)`                     |
| Dynamic dispatch | Vtable lookup                  | Compile-time type collection       |

---

## Proposal

### 1. Interface Declaration

**Core Rule**: Directly write the interface name in the type definition, no `impl` keyword needed.

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type declares it implements the interface
Dog: Type = {
    x: Int,
    Animal,  # Interface declaration
}
```

**Compiler Processing**:

1. Recognize `Animal` as an interface type
2. Check if `Dog` has all methods required by `Animal`
3. If passed → Generate implementation proof
4. If failed → Compile error

**Syntax Sugar Equivalent**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Equivalent to expanding Animal's methods, but preserves origin marker
}

# Equivalent to (but preserves origin information)
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # From Animal
}
```

**Why Origin Markers Are Needed**:

- Direct expansion loses origin information
- Origin markers are used to generate implementation proofs
- At runtime, the proof is used to find the correct method

### 2. Method Implementation

**Core Rule**: Method implementations are supported in both internal and external declarations.

#### 2.1 Internal Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # Method implementation inside
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,
}

# Method implementation outside
Dog.speak: (Self) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # Some methods inside
}

# Some methods outside
Dog.play: (Self) -> Void = { ... }
```

**Compiler Processing**:

1. Collect all definitions (internal and external)
2. Group by signature (overloading)
3. Check for overriding (report error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Overriding

**Core Rules**:

- Different signatures → Overloading → Allowed
- Same signature → Overriding → Error

#### 3.1 Overloading (Allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 Overriding (Prohibited)

```yaoxiang
# Exactly same signature, overriding prohibited
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ Error: overriding not allowed
```

**Error Message**:

```
Error: Dog.speak(Self) -> String duplicate definition
  --> file2:5:1
  |
5 | Dog.speak: (Self) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (Self) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ First definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overloading/overriding rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# External declaration (overloading, allowed)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# External declaration (overriding, prohibited)
Dog.speak: (Self) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core Rule**: Directly write `= value` after the field, no constructor needed.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal,
}
```

**Compiler-Generated Constructors**:

```yaoxiang
# All fields have defaults → Generate no-argument constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have defaults → Generate partial-parameter constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# All-parameter constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External Declaration of Default Values**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# External declaration of defaults
Dog.x: Int = 10
Dog.y: Int = 20
```

**Equivalent to internal declaration**.

### 5. Compiler Implementation

#### 5.1 Interface Descriptor

```rust
// Compiler internal: Interface descriptor
struct InterfaceDescriptor {
    name: String,
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internal: Type definition
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_implementations: Vec<InterfaceImplementation>,
}

// Interface implementation (preserves origin information)
struct InterfaceImplementation {
    interface: InterfaceId,
    methods: HashMap<MethodId, FunctionBody>,
}
```

#### 5.3 Implementation Proof

```rust
// Compiler internal: Implementation proof
struct ImplementationProof {
    type_id: TypeId,
    interface_id: InterfaceId,
    methods: Vec<MethodPointer>,
}
```

#### 5.4 Compilation Flow

```
1. Parse type definitions, collect interface declarations
2. Collect all method definitions (internal and external)
3. Group by signature (overloading)
4. Check for overriding (report error)
5. Check interface completeness
6. Generate implementation proof
7. At runtime, values carry implementation proofs
```

### 6. Dynamic Dispatch

**Core Design**: Compile-time type collection + interface matching, no vtable.

#### 6.1 Heterogeneous Container

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type definitions
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# Heterogeneous container
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

#### 6.2 Compile-Time Type Collection

**Core Strategy: Ownership tracking, incremental construction.** Not scanning all types that implement the interface at compile time—but incrementally collecting at each ownership operation point of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → extends to { Dog, Cat }
animals.append(Bird.new())                 // Further extends to { Dog, Cat, Bird }
```

**Compiler Processing** (incremental):

1. First encounter of `List(I)` being constructed → Generate initial enum (all constructible types known within current compilation unit)
2. Each `append` / `push` / index assignment → Check if value type is already in enum; if not, extend enum variant
3. Generate monomorphized `match` dispatch code for final enum
4. Cross compilation unit: Merge enum variant sets from each unit at link time

**Auto-Generated Enum**:

```yaoxiang
# Compiler auto-generates (invisible to user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) internals are equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key Insight**: Interface matching is a compile-time check, even if types come from dynamically loaded plugins.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler checks: return type of plugin.create_bird() must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check

# Put into heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) Verify bird implements Animal (2) Extend enum
```

**Compiler Processing**:

1. Check the return type of `append` argument
2. Verify that type implements the target interface
3. If passed → Extend enum, allow insertion
4. If failed → Compile error

#### 6.4 Runtime Dispatch

**Call Flow** (compile-time enum match, ImplementationProof already erased):

```
animals[0].speak()
  ↓
Compiler-generated match:
  match animals[0] {
    AnimalGroup.Dog(d) => d.speak(),
    AnimalGroup.Cat(c) => c.speak(),
    AnimalGroup.Bird(b) => b.speak(),
  }
```

**Comparison with Vtable**:

|                     | Vtable (Rust)              | Compile-time Enum (YaoXiang)                    |
| ------------------- | --------------------------- | ----------------------------------------------- |
| Lookup method       | Vtable pointer → method pointer | Enum match → direct call                    |
| Runtime overhead    | One level of indirection    | String comparison/branch (can be optimized by CPU branch prediction) |
| Generated at        | Vtable                      | Enum + match                                    |
| User annotation     | Required `dyn Trait + 'a`   | Not required                                    |
| ImplementationProof | Not applicable              | Erased at compile time, does not exist at runtime |

**YaoXiang's Advantages**:

- No lifetime annotation needed
- Compile-time type safety
- Transparent to user (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit**: Fully supported. Ownership tracking covers all `append`/construction points, enum incrementally built.

**Cross compilation unit**:
Merge enum variant sets from each unit at link time. Design shares mechanism with link-time monomorphization (each unit generates partial enum, linker merges).

**Not supported:** Runtime dynamic types (pure duck typing). Type set is fully known at compile time.

---

## Use Case Analysis

### Basic Interface Implementation

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",
}

# Usage
dog = Dog.new()
dog.speak()  # "Woof"
```

### Multiple Interface Implementation

```yaoxiang
# Multiple interfaces
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    name: (Self) -> String,
}

# Type implements multiple interfaces
Dog: Type = {
    x: Int = 10,
    Animal,
    Pet,
    speak: (Self) -> String = "Woof",
    name: (Self) -> String = "Buddy",
}

# Usage
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### Generic Interfaces

```yaoxiang
# Generic interface
Container: (T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# Implement generic interface
IntList: Type = {
    data: Array(Int),
    Container(Int),
    add: (self: &mut Self, item: Int) -> Void = ...,
    get: (self: &Self, index: Int) -> Int = ...,
}
```

### Heterogeneous Container

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type definitions
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# Heterogeneous container
animals: List(Animal) = [Dog.new(), Cat.new()]

# Usage
for animal in animals {
    print(animal.speak())
}
# Output:
# Woof
# Meow
```

### Plugin System

```yaoxiang
# Interface definition
Plugin: Type = {
    name: (Self) -> String,
    execute: (Self) -> Void,
}

# Main program
main: () -> Void = {
    # Load plugins
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # Compiler checks: plugin1 and plugin2 must implement Plugin interface
    plugins: List(Plugin) = [plugin1, plugin2]

    # Execute all plugins
    for plugin in plugins {
        print(plugin.name())
        plugin.execute()
    }
}
```

---

## Trade-offs

### Advantages

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementations supported both internally and externally
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero-overhead**: No vtable, compile-time type collection
6. **Type-safe**: Interface matching is a compile-time check
7. **Transparent to user**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitation**: Does not support runtime dynamic types (pure duck typing)
2. **Compile-time overhead**: Need to generate enum variants and match dispatch code for each interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: Supported through compile-time interface matching checks
2. **Type set**: Ownership tracking, incremental construction—collecting at each `append`/construction point, not global scanning
3. **Cross compilation unit**: Merge enum variant sets at link time, sharing mechanism with link-time monomorphization

---

## Alternative Approaches

| Approach               | Why Not Chosen              |
| ---------------------- | --------------------------- |
| `impl` keyword         | Adds syntactic complexity   |
| Vtable (`dyn Trait`)   | Requires lifetime annotation (`'a`) |
| Pure duck typing       | Runtime overhead, not type-safe |
| Manual enum wrapping   | Heavy user burden           |

---

## Relationship with RFC-009

**Brands and interface implementation**:

- Interface implementation is at the type layer, not involving brands
- Brands are at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:

- Dynamic dispatch uses implementation proofs, not brand annotations
- Implementation proofs are generated at compile time, zero runtime lookup
- Avoids the complexity of `dyn Trait + 'a`

## Interface Inheritance

Interfaces can include other interfaces. **No new syntax introduced**—uses the exact same syntactic position as type declarations for interfaces:

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet inherits Animal — no new keyword
    name: (Self) -> String,
}

# When Dog implements Pet, it must satisfy all methods of both Animal and Pet
Dog: Type = {
    x: Int,
    Pet,
    speak: (Self) -> String = "Woof",  # From Animal
    name: (Self) -> String = "Buddy",  # From Pet
}
```

**Design Principle:**
Inheritance exists but is not encouraged for overuse. The primary composition method is through multiple interface declarations (`Dog: Type = { Animal, Pet, ... }`). A type can directly declare all interfaces it satisfies without expressing through an inheritance tree. Interface inheritance is used only when there is a clear "is-a" hierarchy.

**Compiler Processing:** Expand the inheritance chain. `Pet` expands to `{ all methods of Animal, name: ... }`. When `Dog` declares `Pet`, the compiler verifies `Dog` satisfies all methods of both `Animal` and `Pet`.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types can choose to override or inherit the default implementation:

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # Must implement
    debug: (Self) -> String = Self.display(),       # ✅ Reference to same-interface method
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ Compile error: Self.name not in fmt
}
```

**Core Constraint: Interfaces cannot assume parent implementations.**
Default methods can only reference methods declared in the same interface. Concrete type fields or methods from other interfaces are invisible to default methods—an interface is a closed contract, and cannot reach into the implementing type's pockets. Violating this constraint reports an error **at interface definition time**.

**Inheritance can assume child implementations:** When interface `Pet` inherits `Animal`, `Pet`'s default methods can use `Animal`'s declared methods—because it inherits, they are guaranteed to exist.

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                                              # Inheritance
    name: (Self) -> String,
    introduce: (Self) -> String = Self.name() + " says " + Self.speak(),  # ✅ speak comes from inherited Animal
}
```

**Compile-Time Behavior:** When a type implements an interface, for each method:

1. Type provides it → Use type's method
2. Type doesn't provide, interface has default → Compiler inlines default implementation into type (zero vtable overhead)
3. Type doesn't provide, interface has no default → Compile error

**Design Principle:** Default methods are similar to `Copy`/`Clone`'s auto-derive mechanism—the compiler automatically generates when needed, user can override. No `virtual`/`override`/`super` keywords introduced.

---

## Implementation Phases

| Phase    | Content                         | Dependency |
| -------- | ------------------------------- | ---------- |
| Phase 1  | Interface declaration syntax    | RFC-011    |
| Phase 2  | Method implementation (internal/external) | Phase 1 |
| Phase 3  | Overloading and overriding rules | Phase 2  |
| Phase 4  | Default value syntax            | Phase 2    |
| Phase 5  | Interface inheritance           | Phase 3    |
| Phase 6  | Default method implementation   | Phase 5    |
| Phase 7  | Implementation proof generation | Phase 6  |
| Phase 8  | Compile-time type collection    | Phase 7    |
| Phase 9  | Dynamic dispatch implementation | Phase 8    |

---

## Design Decision Record

| Decision                   | Decision                                               | Reason                                                                                     | Date       |
| -------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ---------- |
| Interface declaration syntax | Directly write interface name in type body            | Eliminate `impl` keyword, interface declaration is a natural part of type definition       | 2026-06-14 |
| Dynamic dispatch           | Compile-time type collection + auto enum generation    | No vtable, zero runtime lookup, transparent to user                                         | 2026-06-14 |
| External method declaration | Supported                                             | Flexibility equivalent to internal declaration, compiler handles cross-file collection       | 2026-06-14 |
| Overriding                | Prohibited (same signature reports error)               | Overriding leads to unpredictable behavior, overloading covers all cases                   | 2026-06-14 |
| Interface inheritance      | Supported, no new syntax                               | Same syntactic position as type declarations. Encourage composition (multiple interfaces), not deep inheritance trees | 2026-07-03 |
| Default method implementation | Supported, similar to Copy/Clone auto-derive         | Interface provides body, compiler inlines into implementing type as needed; user can override. No virtual/override introduced | 2026-07-03 |
| Default method constraint   | Verified at interface definition time: can only reference same-interface methods, cannot assume parent | Interface is a closed contract. Inheritance can assume child implementations, but interface cannot assume implementing type's fields/methods | 2026-07-03 |
| Type collection strategy    | Ownership tracking, incremental construction — collecting at each append/construction point | Not globally scanning all implementers, but incrementally extending enum at ownership operation points | 2026-07-03 |
| ImplementationProof        | Pure compile-time concept, erased at runtime          | At runtime dispatch via enum match, proof only used for compile-time verification          | 2026-07-03 |
| Cross compilation unit      | Merge enum variants from each unit at link time       | Shares mechanism with link-time monomorphization, each unit generates partial enum, linker merges | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → Supported, no new syntax. `Pet: Type = { Animal, ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~
      → Supported, similar to Copy auto-derive. Interface provides body, compiler inlines as needed
- [ ] Advanced interface constraint usage (associated types, GATs)
- [ ] Interaction with closures (closures implementing interfaces)

---

## References

- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Disposition

| Status         | Location                     | Description         |
| -------------- | ---------------------------- | ------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion |
