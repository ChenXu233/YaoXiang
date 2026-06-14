---
title: "RFC-011a: Interface Implementation and Dynamic Dispatch"
status: "Draft"
author: "Chenxu"
created: "2026-06-14"
updated: "2026-06-14"
group: "rfc-011"
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portion of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defines the generics system, but does not detail the interface implementation mechanism. This document supplements:

1. **Interface Declaration**: The interface name is written directly within the type definition; no `impl` keyword is required
2. **Method Implementation**: Both internal and external declarations are supported
3. **Overloading Rules**: Overloading is allowed when signatures differ; identical signatures cause an error (override prohibited)
4. **Default Values**: Write `= value` directly after a field
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
- ❌ No override (unified overloading rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defines the generics system, but does not detail:

| Problem | Description |
|---------|-------------|
| Interface declaration syntax | How to declare that a type implements an interface? |
| Method implementation location | Internal or external declaration? |
| Overloading rules | How are methods with the same name handled? |
| Default value syntax | How to set default values for fields? |
| Dynamic dispatch | How to implement heterogeneous containers? |

### Design Goals

1. **Concise**: No `impl` keyword required
2. **Flexible**: Both internal and external method implementation are supported
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero Overhead**: No vtable, compile-time type collection

### Comparison with Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Interface declaration | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| Method implementation | Inside `impl` block | Internal or external |
| Overloading | Not supported | Supported (different signatures) |
| Default value | Requires `#[default]` | Write `= value` directly |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| Dynamic dispatch | Vtable lookup | Compile-time type collection |

---

## Proposal

### 1. Interface Declaration

**Core Rule**: Write the interface name directly within the type definition; no `impl` keyword is required.

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
1. Recognize that `Animal` is an interface type
2. Check whether `Dog` has all the methods required by `Animal`
3. If passed → generate implementation proof
4. If failed → compile error

**Syntactic Sugar Equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Equivalent to expanding Animal's methods, but retaining source marking
}

# Equivalent to (but retains source information)
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # From Animal
}
```

**Why Source Marking Is Needed**:
- Direct expansion would lose source information
- Source marking is used to generate implementation proof
- The runtime uses the proof to find the correct method

### 2. Method Implementation

**Core Rule**: Both internal and external method declarations are supported.

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
3. Check for override (error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Override

**Core Rule**:
- Different signatures → overloading → allowed
- Identical signatures → override → error

#### 3.1 Overloading (Allowed)

```yaoxiang
# Different parameter types, overloading is allowed
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (Prohibited)

```yaoxiang
# Identical signatures, override is prohibited
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error Message**:

```
Error: Duplicate definition of Dog.speak(Self) -> String
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

**Internal and external declarations follow the same overloading/override rules**:

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

**Core Rule**: Write `= value` directly after a field, eliminating the need for a constructor.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal,
}
```

**Compiler-Generated Constructors**:

```yaoxiang
# All fields have default values → generate no-argument constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial-argument constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-argument constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External Declaration of Default Values**:

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
1. Parse type definitions, collect interface declarations
2. Collect all method definitions (internal and external)
3. Group by signature (overloading)
4. Check for override (error)
5. Check interface completeness
6. Generate implementation proof
7. At runtime, values carry the implementation proof
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

**Compiler Processing**:

```
1. Scan all types placed into List(Animal)
2. Collect: Dog, Cat
3. Automatically generate the AnimalGroup enum
4. Generate monomorphized code for AnimalGroup
5. At runtime, use enum matching for dispatch
```

**Automatically Generated Enum**:

```yaoxiang
# Automatically generated by the compiler (invisible to the user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
}

# List(Animal) is equivalent to List(AnimalGroup)
animals: List(AnimalGroup) = [
    AnimalGroup.Dog(Dog.new()),
    AnimalGroup.Cat(Cat.new()),
]
```

#### 6.3 Interface Matching Check

**Key Insight**: Interface matching is checked at compile time, even when the type comes from a dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: the return type of plugin.create_bird() must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check

# Place into heterogeneous container
animals: List(Animal) = [Dog.new(), Cat.new(), bird]
```

**Compiler Processing**:
1. Check the return type of `plugin.create_bird()`
2. Verify whether the type implements the `Animal` interface
3. If passed → allow placement into `List(Animal)`
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call Flow**:

```
animals[0].speak()
  ↓
Find the implementation proof of animals[0] (Animal interface)
  ↓
Find the pointer to the speak method from the proof
  ↓
Invoke the method
```

**Comparison with Vtable**:

| | Vtable (Rust) | Implementation Proof (YaoXiang) |
|---|---|---|
| Lookup method | Vtable pointer → method pointer | Implementation proof → method pointer |
| Runtime overhead | One indirect addressing | One indirect addressing |
| Compile-time generation | Vtable | Implementation proof |
| Brand annotation | Requires `dyn Trait + 'a` | Not required |

**YaoXiang's Advantages**:
- No brand annotation required (implementation proof does not need `'a`)
- Compile-time type safety (interface matching is checked at compile time)
- Transparent to the user (no need to write `dyn Animal`)

#### 6.5 Limitations

**No support for fully runtime dynamic types**:
- The type set must be completely known at compile time
- Plugin systems require compile-time interface matching checks
- No support for full duck typing (runtime method existence checks)

**Type set explosion is not an issue**:
- Only linear type collection is required
- Generate an enum variant for each type
- No need to generate code for every combination

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

# Implementing a generic interface
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
2. **Flexible**: Both internal and external method implementation are supported
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero Overhead**: No vtable, compile-time type collection
6. **Type Safe**: Interface matching is checked at compile time
7. **Transparent to User**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitation**: No support for runtime dynamic types (full duck typing)
2. **Compile-time Overhead**: Implementation proof must be generated for each interface
3. **Type Set**: Must be completely known at compile time

### Mitigations

1. **Plugin System**: Supported via compile-time interface matching checks
2. **Compile-time Overhead**: Implementation proof is a lightweight data structure
3. **Type Set**: Linear collection, not exponential explosion

---

## Alternatives

| Approach | Why Not Chosen |
|----------|----------------|
| `impl` keyword | Adds syntactic complexity |
| Vtable (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing | Runtime overhead, not type safe |
| Manual enum wrapping | Heavy burden on the user |

---

## Relationship with RFC-009

**Brand and Interface Implementation**:
- Interface implementation is at the type layer, not involving brand
- Brand is at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic Dispatch and Brand**:
- Dynamic dispatch uses implementation proof, no brand annotation required
- Implementation proof is generated at compile time, zero lookup at runtime
- Avoids the complexity of `dyn Trait + 'a`

---

## Implementation Phases

| Phase | Content | Dependencies |
|-------|---------|--------------|
| Phase 1 | Interface declaration syntax | RFC-011 |
| Phase 2 | Internal/external method implementation | Phase 1 |
| Phase 3 | Overloading and override rules | Phase 2 |
| Phase 4 | Default value syntax | Phase 2 |
| Phase 5 | Implementation proof generation | Phase 3 |
| Phase 6 | Compile-time type collection | Phase 5 |
| Phase 7 | Dynamic dispatch implementation | Phase 6 |

---

## Open Questions

- [ ] Interface inheritance (interfaces can inherit from other interfaces)
- [ ] Default method implementation (interfaces can provide default implementations)
- [ ] Advanced interface constraint usage (associated types, GAT)
- [ ] Interaction with closures (closures implementing interfaces)

---

## References

- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission review |