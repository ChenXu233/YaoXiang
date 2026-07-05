---
title: "RFC-011a: Interface Implementation and Dynamic Dispatch"
status: "Under Review"
author: "晨煦"
created: "2026-06-14"
updated: "2026-07-05"
group: "rfc-011"
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portion of RFC-011 §2.1-2.4.**

## Abstract

RFC-011 defined the generics system but did not elaborate on the interface implementation mechanism. This document supplements:

1. **Interface declaration**: Write the interface name directly in the type definition, no `impl` keyword needed
2. **Method implementation**: Both internal and external declarations are supported
3. **Overload rules**: Different signatures allow overloading; identical signatures raise an error (override forbidden)
4. **Default values**: Write `= value` directly after the field
5. **Dynamic dispatch**: Compile-time type collection + interface matching, no vtable

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

**Eliminated complexity**:
- ❌ No `impl` keyword
- ❌ No `dyn Trait + 'a` annotation
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (overload rules unified)

---

## Motivation

### Gaps in RFC-011

RFC-011 defined the generics system but did not elaborate on:

| Problem | Description |
|------|------|
| Interface declaration syntax | How to declare that a type implements an interface? |
| Method implementation location | Internal declaration or external declaration? |
| Overload rules | How to handle same-named methods? |
| Default value syntax | How to set default values for fields? |
| Dynamic dispatch | How to implement heterogeneous containers? |

### Design Goals

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementation supports both internal and external
3. **Unified**: Overload rules are consistent
4. **Convenient**: Default value syntax is concise
5. **Zero overhead**: No vtable, compile-time type collection

### Comparison with Rust

| Feature | Rust | YaoXiang |
|------|------|----------|
| Interface declaration | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| Method implementation | Inside `impl` block | Internal or external |
| Overloading | Not supported | Supported (different signatures) |
| Default values | Requires `#[default]` | Just write `= value` |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| Dynamic dispatch | Vtable lookup | Compile-time type collection |

---

## Proposal

### 1. Interface Declaration

**Core rule**: Write the interface name directly in the type definition, no `impl` keyword needed.

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

**Compiler handling**:
1. Recognize that `Animal` is an interface type
2. Check whether `Dog` has all methods required by `Animal`
3. If passed → generate implementation proof
4. If failed → compilation error

**Syntactic sugar equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Equivalent to expanding Animal's methods, but retains the source marker
}

# Equivalent to (but retains source information)
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # From Animal
}
```

**Why a source marker is needed**:
- Direct expansion would lose source information
- Source marker is used to generate implementation proof
- At runtime, the proof is used to find the correct method

### 2. Method Implementation

**Core rule**: Method implementation supports both internal and external declarations.

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

**Compiler handling**:
1. Collect all definitions (internal and external)
2. Group by signature (overloading)
3. Check for override (raise error if found)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Override

**Core rules**:
- Different signatures → overloading → allowed
- Identical signatures → override → error

#### 3.1 Overloading (allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (forbidden)

```yaoxiang
# Identical signatures, override forbidden
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error message**:

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

**Internal declarations and external declarations follow the same overloading/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# External declaration (overload, allowed)
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# External declaration (override, forbidden)
Dog.speak: (Self) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core rule**: Write `= value` directly after the field, eliminating the need for a constructor.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal,
}
```

**Compiler-generated constructors**:

```yaoxiang
# All fields have defaults → generate no-arg constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have defaults → generate partial-arg constructor
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-arg constructor
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
1. Parse type definition, collect interface declarations
2. Collect all method definitions (internal and external)
3. Group by signature (overloading)
4. Check for override (raise error)
5. Check interface completeness
6. Generate implementation proof
7. At runtime, values carry the implementation proof
```

### 6. Dynamic Dispatch

**Core design**: Compile-time type collection + interface matching, no vtable.

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

**Core strategy: ownership tracking, incremental construction.** Rather than scanning all types that implement the interface at compile time, the type set is collected incrementally at each **ownership operation point** of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → extends to { Dog, Cat }
animals.append(Bird.new())                 // Extends again: { Dog, Cat, Bird }
```

**Compiler handling (incremental)**:

1. Encounter `List(I)` constructed for the first time → generate initial enum (all constructor types known within the current compilation unit)
2. On each `append` / `push` / index assignment → check whether the value's type is already in the enum; if not, extend the enum variant
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: link-time merging of each unit's enum variant set

**Auto-generated enum**:

```yaoxiang
# Compiler-generated automatically (invisible to the user)
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

# Compiler check: plugin.create_bird()'s return type must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check

# Put into heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler handling**:
1. Check the return type of the `append` argument
2. Verify that the type implements the target interface
3. If passed → extend enum, allow insertion
4. If failed → compilation error

#### 6.4 Runtime Dispatch

**Call flow (compile-time enum match; ImplementationProof has been erased)**:

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

**Comparison with vtable**:

| | Vtable (Rust) | Compile-time enum (YaoXiang) |
|---|---|---|
| Lookup method | Vtable pointer → method pointer | Enum match → direct call |
| Runtime overhead | One indirection | String comparison / branch (optimizable by CPU branch prediction) |
| Compile-time generation | Vtable | Enum + match |
| User annotation | Requires `dyn Trait + 'a` | Not required |
| ImplementationProof | N/A | Compile-time erased; does not exist at runtime |

**YaoXiang's advantages**:
- No brand annotation needed
- Compile-time type safety
- Transparent to users (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit:** Fully supported. Ownership tracking covers all `append`/construction points; the enum is built incrementally.

**Cross-compilation-unit:** Link-time merging of each unit's enum variant set. The design shares the same mechanism as link-time monomorphization (each unit generates a partial enum, the linker merges).

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

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementation supports both internal and external
3. **Unified**: Overload rules are consistent
4. **Convenient**: Default value syntax is concise
5. **Zero overhead**: No vtable, compile-time type collection
6. **Type safe**: Interface matching is a compile-time check
7. **Transparent to users**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitations**: Runtime dynamic typing (full duck typing) is not supported
2. **Compile-time overhead**: Need to generate enum variants and match dispatch code for each interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigation

1. **Plugin system**: Supported via compile-time interface matching check
2. **Type set**: Ownership tracking, incremental construction — collected at each `append`/construction point, not a global scan
3. **Cross-compilation-unit**: Link-time merging of enum variant sets, sharing the mechanism with link-time monomorphization

---

## Alternatives

| Alternative | Why not chosen |
|------|--------------|
| `impl` keyword | Increases syntax complexity |
| Vtable (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing | Runtime overhead, not type safe |
| Manual enum wrapping | Heavy user burden |

---

## Relationship with RFC-009

**Brand and interface implementation**:
- Interface implementation is at the type layer, not involving brand
- Brand is at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brand**:
- Dynamic dispatch uses implementation proof, no brand annotation needed
- Implementation proof is generated at compile time with zero runtime lookup
- Avoids the complexity of `dyn Trait + 'a`

## Interface Inheritance

Interfaces can include other interfaces. **No new syntax introduced** — uses exactly the same syntactic position as a type declaring an interface:

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

**Design principle:** Inheritance exists, but its abuse is discouraged. The primary composition pattern is via multiple interface declarations (`Dog: Type = { Animal, Pet, ... }`). A type can directly declare all interfaces it satisfies, without needing to express that through an inheritance tree. Interface inheritance is only used when there is a clear "is-a" hierarchy.

**Compiler handling:** Expand the inheritance chain. `Pet` expands to `{ all methods of Animal, name: ... }`. When `Dog` declares `Pet`, the compiler verifies that `Dog` satisfies all methods of both `Animal` and `Pet`.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types may choose to override or inherit the default:

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # Must be implemented
    debug: (Self) -> String = Self.display(),       # ✅ References same-interface method
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ Compile error: Self.name is not in fmt
}
```

**Core constraint: an interface cannot assume an upper-level implementation.** A default method can only reference methods declared in the same interface. Fields of the concrete type or methods of other interfaces are invisible to default methods — an interface is a closed contract and cannot reach into the implementing type's pockets. Violations of this constraint are reported as errors **at interface definition time**.

**Inheritance can assume a lower-level implementation:** When interface `Pet` inherits `Animal`, a default method of `Pet` can use methods declared by `Animal` — because they are inherited, their presence is guaranteed.

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

**Compile-time behavior:** When a type implements an interface, for each method:
1. Type provides it → use the type's method
2. Type doesn't provide, interface has default → compiler inlines the default implementation into the type (zero vtable overhead)
3. Type doesn't provide, interface has no default → compilation error

**Design principle:** Default methods resemble the auto-derive mechanism of `Copy`/`Clone` — the compiler generates them on demand, and the user may override. No `virtual`/`override`/`super` keywords are introduced.

---

## Implementation Phases

| Phase | Content | Dependency |
|------|------|------|
| Phase 1 | Interface declaration syntax | RFC-011 |
| Phase 2 | Internal/external method declarations | Phase 1 |
| Phase 3 | Overloading and override rules | Phase 2 |
| Phase 4 | Default value syntax | Phase 2 |
| Phase 5 | Interface inheritance | Phase 3 |
| Phase 6 | Default method implementation | Phase 5 |
| Phase 7 | Implementation proof generation | Phase 6 |
| Phase 8 | Compile-time type collection | Phase 7 |
| Phase 9 | Dynamic dispatch implementation | Phase 8 |

---

## Design Decision Log

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| Interface declaration syntax | Write the interface name directly in the type body | Eliminate the `impl` keyword; interface declaration is a natural part of the type definition | 2026-06-14 |
| Dynamic dispatch | Compile-time type collection + auto-generated enum | No vtable, zero runtime lookup, transparent to users | 2026-06-14 |
| External method declaration | Supported | Equivalent in flexibility to internal declaration; compiler is responsible for cross-file collection | 2026-06-14 |
| Override | Forbidden (same-signature error) | Override leads to unpredictable behavior; overloading covers all cases | 2026-06-14 |
| Interface inheritance | Supported, no new syntax | Same syntactic position as a type declaring an interface. Encourages composition (multiple interface declarations); discourages deep inheritance trees | 2026-07-03 |
| Default method implementation | Supported, similar to auto-derive for Copy/Clone | Interface provides a body; compiler inlines on demand; user may override. No `virtual`/`override` introduced | 2026-07-03 |
| Default method constraint | Validated at interface definition: may only reference same-interface methods; cannot assume an upper-level implementation | The interface is a closed contract. Inheritance can assume a lower-level implementation, but the interface cannot assume the implementing type's fields/methods | 2026-07-03 |
| Type collection strategy | Ownership tracking, incremental construction — collected at each `append`/construction point | Not a global scan of all implementers; incrementally extends the enum at each ownership operation point | 2026-07-03 |
| ImplementationProof | Pure compile-time concept, erased at runtime | Runtime uses enum match dispatch; the proof is used only for compile-time verification | 2026-07-03 |
| Cross-compilation-unit | Link-time merging of each unit's enum variants | Shares the mechanism with link-time monomorphization; each unit generates a partial enum, the linker merges | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → Supported, no new syntax. `Pet: Type = { Animal, ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ → Supported, similar to auto-derive for Copy. Interface provides a body; compiler inlines on demand
- [ ] Advanced uses of interface constraints (associated types, GAT)
- [ ] Interaction with closures (closures implementing interfaces)

---

## References

- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion |