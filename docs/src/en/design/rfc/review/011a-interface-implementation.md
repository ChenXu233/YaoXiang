---
title: "RFC-011a: Interface Implementation and Dynamic Dispatch"
status: "Under Review"
author: "Chenxu"
created: "2026-06-14"
updated: "2026-07-05"
group: "rfc-011"
---

# RFC-011a: Interface Implementation and Dynamic Dispatch  

> **Parent RFC**: [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portions of RFC-011 §2.1-2.4.**

## Abstract

RFC-011 defines the generic type system, but does not detail the interface implementation mechanism. This document supplements:

1. **Interface Declaration**: Write the interface name directly in the type definition, no `impl` keyword required
2. **Method Implementation**: Both internal and external declarations are supported
3. **Overload Rules**: Overloading allowed with different signatures; same signature triggers an error (override prohibited)
4. **Default Values**: Write `= value` directly after the field
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

# External declaration (overload)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Eliminated Complexity**:
- ❌ No `impl` keyword
- ❌ No `dyn Trait + 'a` annotation
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (unified overload rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defines the generic type system but does not detail:

| Problem | Description |
|------|------|
| Interface declaration syntax | How to declare that a type implements an interface? |
| Method implementation location | Internal or external declaration? |
| Overload rules | How to handle methods with the same name? |
| Default value syntax | How to set default values for fields? |
| Dynamic dispatch | How to implement heterogeneous containers? |

### Design Goals

1. **Concise**: No `impl` keyword required
2. **Flexible**: Method implementation in either internal or external form
3. **Unified**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero-cost**: No vtable, compile-time type collection

### Comparison with Rust

| Feature | Rust | YaoXiang |
|------|------|----------|
| Interface declaration | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| Method implementation | In `impl` block | Internal or external |
| Overloading | Not supported | Supported (different signatures) |
| Default values | Requires `#[default]` | Write `= value` directly |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| Dynamic dispatch | Vtable lookup | Compile-time type collection |

---

## Proposal

### 1. Interface Declaration

**Core Rule**: Write the interface name directly in the type definition, no `impl` keyword required.

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type declares interface implementation
Dog: Type = {
    x: Int,
    Animal,  # Interface declaration
}
```

**Compiler Processing**:
1. Identify `Animal` as an interface type
2. Check whether `Dog` has all methods required by `Animal`
3. If passed → generate implementation proof
4. If failed → compile error

**Syntactic Sugar Equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Equivalent to expanding Animal's methods, but retaining the source tag
}

# Equivalent to (but preserves source information)
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # From Animal
}
```

**Why Source Tags Are Needed**:
- Direct expansion loses source information
- Source tags are used to generate implementation proofs
- At runtime, the correct method is located via the proof

### 2. Method Implementation

**Core Rule**: Method implementation supports both internal and external declarations.

#### 2.1 Internal Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # Method implementation internal
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,
}

# Method implementation external
Dog.speak: (Self) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # Some methods internal
}

# Some methods external
Dog.play: (Self) -> Void = { ... }
```

**Compiler Processing**:
1. Collect all definitions (internal and external)
2. Group by signature (overload)
3. Check for overrides (error if found)
4. Check interface completeness
5. Generate implementation proof

### 3. Overload and Override

**Core Rules**:
- Different signatures → overload → allowed
- Same signatures → override → error

#### 3.1 Overload (Allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (Prohibited)

```yaoxiang
# Identical signatures, override prohibited
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error Message**:

```
Error: Duplicate definition of Dog.speak(Self) -> String
  --> file2:5:1
  |
5 | Dog.speak: (Self) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (Self) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ first definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overload/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# External declaration (overload, allowed)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# External declaration (override, prohibited)
Dog.speak: (Self) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core Rule**: Write `= value` directly after the field, eliminating the need for constructor functions.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal,
}
```

**Compiler-Generated Constructors**:

```yaoxiang
# All fields have default values → generate no-arg constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial-arg constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-arg constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External Default Value Declaration**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# External default value declaration
Dog.x: Int = 10
Dog.y: Int = 20
```

Equivalent to internal declaration.

### 5. Compiler Implementation

#### 5.1 Interface Descriptor

```rust
// Compiler internal: interface descriptor
struct InterfaceDescriptor {
    name: String,
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internal: type definition
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_implementations: Vec<InterfaceImplementation>,
}

// Interface implementation (preserves source information)
struct InterfaceImplementation {
    interface: InterfaceId,
    methods: HashMap<MethodId, FunctionBody>,
}
```

#### 5.3 Implementation Proof

```rust
// Compiler internal: implementation proof
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
3. Group by signature (overload)
4. Check for overrides (error if found)
5. Check interface completeness
6. Generate implementation proof
7. At runtime, values carry implementation proof
```

### 6. Dynamic Dispatch

**Core Design**: Compile-time type collection + interface matching, no vtable.

#### 6.1 Heterogeneous Container

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type definition
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

**Core Strategy: Ownership tracking with incremental construction.** Rather than scanning all types implementing the interface at compile time, we incrementally collect at each **ownership operation point** of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → expands to { Dog, Cat }
animals.append(Bird.new())                 // Expands to { Dog, Cat, Bird }
```

**Compiler Processing** (incremental):

1. When `List(I)` is first constructed → generate the initial enum (all constructible types known in the current compilation unit)
2. Each `append` / `push` / index assignment → check whether the value's type is already in the enum; if not, extend the enum variants
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: merge each unit's enum variant set at link time

**Auto-Generated Enum**:

```yaoxiang
# Auto-generated by the compiler (invisible to user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key Insight**: Interface matching is a compile-time check, even for types from dynamically loaded plugins.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: plugin.create_bird() return type must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check

# Place into heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler Processing**:
1. Check the return type of the `append` parameter
2. Verify whether the type implements the target interface
3. If passed → extend enum, allow insertion
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call Flow (compile-time enum match; ImplementationProof has been erased)**:

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

| | Vtable (Rust) | Compile-Time Enum (YaoXiang) |
|---|---|---|
| Lookup method | Vtable pointer → method pointer | Enum match → direct call |
| Runtime overhead | One indirection | String comparison/branch (optimizable by CPU branch prediction) |
| Compile-time generation | Vtable | Enum + match |
| User annotation | Requires `dyn Trait + 'a` | Not required |
| ImplementationProof | Not applicable | Compile-time erased, nonexistent at runtime |

**YaoXiang's Advantages**:
- No brand annotation required
- Compile-time type safety
- Transparent to users (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit:** Full support. Ownership tracking covers all `append`/construction points; the enum is incrementally built.

**Cross-compilation-unit:** Each unit's enum variant set is merged at link time. The design shares the same mechanism as link-time monomorphization (each unit generates a partial enum; the linker merges them).

**Not supported:** Runtime dynamic typing (full duck typing). The type set is fully known at compile time.
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

### Generic Interface

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

# Type definition
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

    # Compiler check: plugin1 and plugin2 must implement the Plugin interface
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

1. **Concise**: No `impl` keyword required
2. **Flexible**: Method implementation in either internal or external form
3. **Unified**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero-cost**: No vtable, compile-time type collection
6. **Type-safe**: Interface matching is a compile-time check
7. **Transparent to users**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitations**: No runtime dynamic typing (full duck typing)
2. **Compile-time overhead**: Need to generate enum variants and match dispatch code for each interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: Supported via compile-time interface matching checks
2. **Type set**: Ownership tracking with incremental construction — collected at each `append`/construction point, not via global scan
3. **Cross-compilation-unit**: Merge enum variant sets at link time, sharing the mechanism with link-time monomorphization

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| `impl` keyword | Increases syntax complexity |
| Vtable (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing | Runtime overhead, not type-safe |
| Manual enum wrapping | Heavy user burden |

---

## Relationship with RFC-009

**Brands and Interface Implementation**:
- Interface implementation lives at the type layer, not involving brands
- Brands live at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic Dispatch and Brands**:
- Dynamic dispatch uses implementation proofs, no brand annotation required
- Implementation proofs are generated at compile time, zero lookup at runtime
- Avoids the complexity of `dyn Trait + 'a`


## Interface Inheritance

Interfaces can include other interfaces. **No new syntax introduced**—uses the exact same syntactic position as type interface declarations:

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet inherits Animal — no new keyword
    name: (Self) -> String,
}

# When Dog implements Pet, it must satisfy all methods from both Animal and Pet
Dog: Type = {
    x: Int,
    Pet,
    speak: (Self) -> String = "Woof",  # From Animal
    name: (Self) -> String = "Buddy",  # From Pet
}
```

**Design Principle:** Inheritance exists but is not encouraged for overuse. The primary composition approach is through multiple interface declarations (`Dog: Type = { Animal, Pet, ... }`). A type can directly declare all interfaces it satisfies without needing to express it through an inheritance tree. Interface inheritance is only used when there is a clear "is-a" hierarchy.

**Compiler Processing:** Expand the inheritance chain. `Pet` expands to `{ all methods from Animal, name: ... }`. When `Dog` declares `Pet`, the compiler verifies that `Dog` satisfies all methods from both `Animal` and `Pet`.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types can choose to override or inherit the default:

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # Must implement
    debug: (Self) -> String = Self.display(),       # ✅ References same-interface method
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ Compile error: Self.name not in fmt
}
```

**Core Constraint: Interfaces cannot assume supertype implementations.** Default methods can only reference methods declared in the same interface. Fields of concrete types or methods of other interfaces are invisible to default methods—an interface is a closed contract and cannot reach into the implementing type's pockets. Violations of this constraint trigger an error at **interface definition time**.

**Inheritance Can Assume Subtype Implementations:** When interface `Pet` inherits `Animal`, the default methods of `Pet` can use methods declared by `Animal`—because it's inherited, so it's guaranteed to exist.

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                                              # Inheritance
    name: (Self) -> String,
    introduce: (Self) -> String = Self.name() + " says " + Self.speak(),  # ✅ speak from inherited Animal
}
```

**Compile-Time Behavior:** When a type implements an interface, for each method:
1. Type provides it → use the type's method
2. Type does not provide, interface has default → compiler inlines the default implementation into the type (zero vtable overhead)
3. Type does not provide, interface has no default → compile error

**Design Principle:** Default methods are similar to the auto-derive mechanism for `Copy`/`Clone`—the compiler auto-generates them when needed, and users can override. No `virtual`/`override`/`super` keywords introduced.
---

## Implementation Phases

| Phase | Content | Dependencies |
|------|------|------|
| Phase 1 | Interface declaration syntax | RFC-011 |
| Phase 2 | Internal/external method implementation | Phase 1 |
| Phase 3 | Overload and override rules | Phase 2 |
| Phase 4 | Default value syntax | Phase 2 |
| Phase 5 | Interface inheritance | Phase 3 |
| Phase 6 | Default method implementation | Phase 5 |
| Phase 7 | Implementation proof generation | Phase 6 |
| Phase 8 | Compile-time type collection | Phase 7 |
| Phase 9 | Dynamic dispatch implementation | Phase 8 |

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| Interface declaration syntax | Write interface name directly in type body | Eliminate `impl` keyword; interface declaration is a natural part of type definition | 2026-06-14 |
| Dynamic dispatch | Compile-time type collection + auto enum generation | No vtable, zero runtime lookup, transparent to users | 2026-06-14 |
| External method declaration | Supported | Equivalent flexibility to internal declaration; compiler handles cross-file collection | 2026-06-14 |
| Override | Prohibited (same signature triggers error) | Override causes unpredictable behavior; overloading covers all cases | 2026-06-14 |
| Interface inheritance | Supported, no new syntax | Same syntactic position as type interface declarations. Encourages composition (multi-interface declaration), discourages deep inheritance trees | 2026-07-03 |
| Default method implementation | Supported, similar to Copy/Clone auto-derive | Interface provides body, compiler inlines on the implementing type; users can override. No virtual/override introduced | 2026-07-03 |
| Default method constraints | Verify at interface definition: can only reference same-interface methods, cannot assume supertype implementations | Interface is a closed contract. Inheritance can assume subtype implementations, but interfaces cannot assume fields/methods of implementing types | 2026-07-03 |
| Type collection strategy | Ownership tracking, incremental construction — collected at each append/construction point | Not a global scan of all implementers; incremental enum extension at ownership operation points | 2026-07-03 |
| ImplementationProof | Pure compile-time concept, erased at runtime | Runtime uses enum match dispatch; proof is only for compile-time validation | 2026-07-03 |
| Cross-compilation-unit | Merge each unit's enum variants at link time | Shares mechanism with link-time monomorphization; each unit generates partial enum, linker merges | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → Supported, no new syntax. `Pet: Type = { Animal, ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ → Supported, similar to Copy auto-derive. Interface provides body, compiler inlines as needed
- [ ] Advanced usage of interface constraints (associated types, GAT)
- [ ] Interaction with closures (closures implementing interfaces)

---

## References

- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |