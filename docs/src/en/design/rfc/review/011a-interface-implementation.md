---
title: "RFC-011a: Interface Implementation and Dynamic Dispatch"
status: "Under Review"
author: "晨煦 (Chenxu)"
created: "2026-06-14"
updated: "2026-07-03"
group: "rfc-011"
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint section of RFC-011 §2.1-2.4.**

## Abstract

RFC-011 defines the generic type system but does not detail the interface implementation mechanism. This document supplements:

1. **Interface declaration**: Write the interface name directly in the type definition; no `impl` keyword needed
2. **Method implementation**: Both internal and external declarations are supported
3. **Overloading rules**: Different signatures allow overloading; same signatures cause errors (override forbidden)
4. **Default values**: Write `= value` directly after a field
5. **Dynamic dispatch**: Compile-time type collection + interface matching, no virtual table

**Core design**:

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

**Complexity eliminated**:
- ❌ No `impl` keyword
- ❌ No `dyn Trait + 'a` annotation
- ❌ No virtual table (compile-time type collection + enum wrapping)
- ❌ No override (unified overloading rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defines the generic type system but does not detail:

| Problem | Description |
|------|------|
| Interface declaration syntax | How to declare a type implements an interface? |
| Method implementation location | Internal or external declaration? |
| Overloading rules | How to handle methods with the same name? |
| Default value syntax | How to set default values for fields? |
| Dynamic dispatch | How to implement heterogeneous containers? |

### Design Goals

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementation supported both internally and externally
3. **Unified**: Consistent overloading rules
4. **Convenient**: Simple default value syntax
5. **Zero overhead**: No virtual table, compile-time type collection

### Comparison with Rust

| Feature | Rust | YaoXiang |
|------|------|----------|
| Interface declaration | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| Method implementation | Inside `impl` block | Internal or external |
| Overloading | Not supported | Supported (different signatures) |
| Default values | Requires `#[default]` | Write `= value` directly |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| Dynamic dispatch | Virtual table lookup | Compile-time type collection |

---

## Proposal

### 1. Interface Declaration

**Core rule**: Write the interface name directly in the type definition; no `impl` keyword needed.

```yaoxiang
# Interface definition
Animal: Type = {
    speak: (Self) -> String,
}

# Type declares implementation of interface
Dog: Type = {
    x: Int,
    Animal,  # Interface declaration
}
```

**Compiler processing**:
1. Identify that `Animal` is an interface type
2. Check whether `Dog` has all methods required by `Animal`
3. If passed → generate implementation proof
4. If failed → compile error

**Syntactic sugar equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Equivalent to expanding Animal's methods, but preserving source marker
}

# Equivalent to (but preserving source information)
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # From Animal
}
```

**Why source markers are needed**:
- Direct expansion loses source information
- Source markers are used to generate implementation proof
- The correct method is found at runtime through the proof

### 2. Method Implementation

**Core rule**: Both internal and external method declarations are supported.

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

**Compiler processing**:
1. Collect all definitions (internal and external)
2. Group by signature (overloading)
3. Check for overrides (report error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Override

**Core rule**:
- Different signatures → overloading → allowed
- Same signatures → override → report error

#### 3.1 Overloading (allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (forbidden)

```yaoxiang
# Completely identical signatures, override forbidden
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error message**:

```
error: duplicate definition of Dog.speak(Self) -> String
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

**Internal and external declarations follow the same overloading/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# External declaration (overloading, allowed)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# External declaration (override, forbidden)
Dog.speak: (Self) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core rule**: Write `= value` directly after a field, eliminating the need for constructors.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal,
}
```

**Compiler-generated constructors**:

```yaoxiang
# All fields have default values → generate no-argument constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial-argument constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-argument constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External declaration of default values**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# External declaration of default values
Dog.x: Int = 10
Dog.y: Int = 20
```

**Equivalent to internal declaration**.

### 5. Compiler Implementation

#### 5.1 Interface Descriptor

```rust
// Compiler internals: interface descriptor
struct InterfaceDescriptor {
    name: String,
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internals: type definition
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
// Compiler internals: implementation proof
struct ImplementationProof {
    type_id: TypeId,
    interface_id: InterfaceId,
    methods: Vec<MethodPointer>,
}
```

#### 5.4 Compilation Flow

```
1. Parse type definition, collect interface declarations
2. Collect all method definitions (internal and external)
3. Group by signature (overloading)
4. Check for overrides (report error)
5. Check interface completeness
6. Generate implementation proof
7. At runtime, values carry implementation proof
```

### 6. Dynamic Dispatch

**Core design**: Compile-time type collection + interface matching, no virtual table.

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

**Core strategy: ownership tracking, incremental construction.** Rather than scanning all types that implement the interface at compile time, the collection is performed incrementally at each **ownership operation point** of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → extends to { Dog, Cat }
animals.append(Bird.new())                 // Further extends to { Dog, Cat, Bird }
```

**Compiler processing** (incremental):

1. When `List(I)` is first constructed → generate initial enum (all constructor types known within the current compilation unit)
2. On each `append` / `push` / indexed assignment → check whether the value's type is already in the enum; if not, extend the enum variants
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: merge the enum variant sets from each unit at link time

**Auto-generated enum**:

```yaoxiang
# Auto-generated by the compiler (invisible to the user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) is internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key insight**: Interface matching is a compile-time check, even when the type comes from a dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: the return type of plugin.create_bird() must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check

# Putting it into a heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler processing**:
1. Check the return type of the `append` argument
2. Verify whether the type implements the target interface
3. If passed → extend the enum, allow insertion
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call flow (compile-time enum match, ImplementationProof erased)**:

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

**Comparison with virtual table**:

| | Virtual table (Rust) | Compile-time enum (YaoXiang) |
|---|---|---|
| Lookup method | Virtual table pointer → method pointer | Enum match → direct call |
| Runtime overhead | One indirect addressing | String comparison/branch (optimizable by CPU branch prediction) |
| Compile-time generation | Virtual table | Enum + match |
| User annotation | Requires `dyn Trait + 'a` | Not required |
| ImplementationProof | N/A | Compile-time erasure, does not exist at runtime |

**YaoXiang's advantages**:
- No brand annotation required
- Compile-time type safety
- Transparent to users (no need to write `dyn Animal`)
- ImplementationProof is a purely compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit**: Full support. Ownership tracking covers all `append`/construction points, with incremental enum construction.

**Cross-compilation-unit**: Link-time merging of enum variant sets from each unit. The design shares the same mechanism as link-time monomorphization (each unit generates a partial enum, the linker merges).

**Not supported**: Runtime dynamic types (full duck typing). The type set is fully known at compile time.
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

# Type implementing multiple interfaces
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

# Implementing generic interface
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

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementation supported both internally and externally
3. **Unified**: Consistent overloading rules
4. **Convenient**: Simple default value syntax
5. **Zero overhead**: No virtual table, compile-time type collection
6. **Type safety**: Interface matching is a compile-time check
7. **Transparent to users**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitations**: Does not support runtime dynamic types (full duck typing)
2. **Compile-time overhead**: Need to generate enum variants and match dispatch code for each interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigation Measures

1. **Plugin system**: Supported through compile-time interface matching checks
2. **Type set**: Ownership tracking, incremental construction — collected at each `append`/construction point, not a global scan
3. **Cross-compilation-unit**: Link-time merging of enum variant sets, sharing the same mechanism as link-time monomorphization

---

## Alternative Approaches

| Approach | Why not chosen |
|------|--------------|
| `impl` keyword | Increases syntax complexity |
| Virtual table (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing | Runtime overhead, type unsafe |
| Enum wrapping (manual) | Heavy user burden |

---

## Relationship with RFC-009

**Brands and interface implementation**:
- Interface implementation is at the type layer, not involving brands
- Brands are at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:
- Dynamic dispatch uses implementation proof, no brand annotation needed
- Implementation proof is generated at compile time, with zero runtime lookup
- Avoids the complexity of `dyn Trait + 'a`


## Interface Inheritance

Interfaces can include other interfaces. **No new syntax is introduced** — the same syntactic position used for type-to-interface declarations is used:

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet inherits from Animal — no new keyword
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

**Design principle**: Inheritance exists but its abuse is discouraged. The primary composition approach is through multiple interface declarations (`Dog: Type = { Animal, Pet, ... }`). A type can directly declare all interfaces it satisfies without needing to express them through an inheritance tree. Interface inheritance is used only when there is a clear "is-a" hierarchy.

**Compiler processing**: Expand the inheritance chain. `Pet` expands to `{ all methods of Animal, name: ... }`. When `Dog` declares `Pet`, the compiler verifies that `Dog` satisfies all methods from both `Animal` and `Pet`.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types can choose to override or inherit the default implementation:

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # Must implement
    debug: (Self) -> String = Self.display(),       # ✅ References same-interface method
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ Compile error: Self.name is not in fmt
}
```

**Core constraint: interfaces cannot assume the implementing type's other members.** Default methods can only reference methods already declared in the same interface. The specific type's fields or methods from other interfaces are not visible to default methods — an interface is a closed contract and cannot reach into the implementing type's pockets. Violations of this constraint produce an error **at interface definition time**.

**Inheritance can assume the lower-level implementation:** When interface `Pet` inherits from `Animal`, `Pet`'s default methods can use methods declared in `Animal` — because of the inheritance, their existence is guaranteed.

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

**Compile-time behavior**: When a type implements an interface, for each method:
1. Type provides it → use the type's method
2. Type does not provide, interface has default → compiler inlines the default implementation into the type (zero virtual-table overhead)
3. Type does not provide, interface has no default → compile error

**Design principle**: Default methods resemble the auto-derive mechanism for `Copy`/`Clone` — the compiler auto-generates when needed, and the user can override. No `virtual`/`override`/`super` keywords are introduced.
---

## Implementation Phases

| Phase | Content | Dependencies |
|------|------|------|
| Phase 1 | Interface declaration syntax | RFC-011 |
| Phase 2 | Internal/external method declaration | Phase 1 |
| Phase 3 | Overloading and override rules | Phase 2 |
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
| Interface declaration syntax | Write interface name directly inside the type body | Eliminate the `impl` keyword; interface declaration is a natural part of the type definition | 2026-06-14 |
| Dynamic dispatch | Compile-time type collection + auto-generated enum | No virtual table, zero runtime lookup, transparent to users | 2026-06-14 |
| External method declaration | Supported | Equivalent in flexibility to internal declaration, compiler handles cross-file collection | 2026-06-14 |
| Override | Forbidden (same-signature error) | Override causes unpredictable behavior; overloading covers all cases | 2026-06-14 |
| Interface inheritance | Supported, no new syntax | Same syntactic position as type-to-interface declaration. Encourages composition (multiple interface declarations), discourages deep inheritance trees | 2026-07-03 |
| Default method implementation | Supported, similar to Copy/Clone auto-derive | Interface provides the body, compiler inlines at the implementing type; user can override. No virtual/override introduced | 2026-07-03 |
| Default method constraint | Verified at interface definition: can only reference same-interface methods, cannot assume the implementing type's members | An interface is a closed contract. Inheritance can assume the lower-level implementation, but an interface cannot assume the implementing type's fields/methods | 2026-07-03 |
| Type collection strategy | Ownership tracking, incremental construction — collected at each append/construction point | Not a global scan of all implementers, but incremental enum extension at ownership operation points | 2026-07-03 |
| ImplementationProof | Purely compile-time concept, erased at runtime | Runtime uses enum match dispatch; proof is used only for compile-time verification | 2026-07-03 |
| Cross-compilation-unit | Link-time merging of enum variant sets from each unit | Shares the same mechanism as link-time monomorphization; each unit generates a partial enum, the linker merges | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit from other interfaces)~~ → Supported, no new syntax. `Pet: Type = { Animal, ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ → Supported, similar to Copy auto-derive. Interface provides the body, compiler inlines when needed
- [ ] Advanced interface constraints (associated types, GAT)
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