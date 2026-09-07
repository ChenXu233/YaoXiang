---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'accepted'
author: 'Chenxu'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portion of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defined the generics system but did not elaborate on the interface implementation mechanism.
This document supplements:

1. **Interface declaration**: Interfaces are parameterized types — `(Self: Type) -> Type`, with
   concrete types passed in at implementation
2. **Method implementation**: Both internal and external declarations are supported
3. **Overload rules**: Overloading is allowed when signatures differ; identical signatures produce
   an error (no overriding)
4. **Default values**: Write `= value` directly after the field
5. **Dynamic dispatch**: compile-time type collection + interface matching, no vtable

**Core design**:

```yaoxiang
# Interface definition (parameterized type, Self is an explicit type parameter)
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition (internal declaration)
Dog: Type = {
    x: Int = 10,
    Animal(Dog),  # Interface instantiation, Self ↦ Dog
    speak: (self: &Dog) -> String = "Woof",
}

# External declaration (overload)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Receiver spelling convention** (errata 2026-08-30, aligned with RFC-009 ownership semantics):

- Method receivers follow the signature semantics: `&Self` = borrow (the default interface
  convention — method calls do not consume the receiver), `&mut Self` = mutable borrow, by-value
  `Self` = consume the receiver (Move, RFC-009).
- In the impl-side signature, `Self` is an alias for the impl type: the interface
  `speak: (self: &Self)` matches both impl `(self: &Dog)` / `(self: &Self)` (fully identical after
  Self↦impl type substitution, §3).
- The by-value receiver spelling in historical examples (`(self: Self)`) meant borrow; this document
  has unified them to explicit `&Self`; the by-value spelling now reserves the "consume" semantics
  and is no longer used interchangeably.

**Complexity eliminated**:

- ❌ No `impl` keyword
- ❌ No `Self` magic keyword (`Self` is an explicit type parameter, no different from `T`)
- ❌ No `dyn Trait + 'a` annotation
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No overriding (uniform overload rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defined the generics system but did not elaborate on:

| Problem                        | Description                                             |
| ------------------------------ | ------------------------------------------------------- |
| Interface declaration syntax   | How is it declared that a type implements an interface? |
| Method implementation location | Internal declaration or external declaration?           |
| Overload rules                 | How are same-named methods handled?                     |
| Default value syntax           | How do fields set default values?                       |
| Dynamic dispatch               | How are heterogeneous containers implemented?           |

### Design Goals

1. **Concise**: No `impl` keyword needed
2. **Flexible**: Method implementation supported both internally and externally
3. **Uniform**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero overhead**: No vtable, compile-time type collection

### Comparison with Rust

| Feature                 | Rust                                   | YaoXiang                            |
| ----------------------- | -------------------------------------- | ----------------------------------- |
| Interface declaration   | `impl Animal for Dog { ... }`          | `Dog: Type = { Animal(Dog), ... }`  |
| Method implementation   | In `impl` blocks                       | Internal or external                |
| Overload                | Not supported                          | Supported (different signatures)    |
| Default values          | Requires `#[default]`                  | Write `= value` directly            |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>`            | `List(Animal)`                      |
| Dynamic dispatch        | Vtable lookup                          | compile-time type collection        |
| Self keyword            | Magic keyword, implicit quantification | Explicit type parameter, equal to T |

---

## Proposal

### 1. Interface Declaration

**Core rule**: An interface is a parameterized type `(Self: Type) -> Type`, where `Self` is an
explicit type parameter, not a magic keyword. At implementation, the interface is called with a
concrete type passed in.

```yaoxiang
# Interface definition (completely consistent with RFC-011 generics types)
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type declaration implements interface
Dog: Type = {
    x: Int,
    Animal(Dog),  # Instantiate interface, Self ↦ Dog
}
```

**Compiler handling**:

1. Recognize `Animal(Dog)` as an instantiation call of `(Self: Type) -> Type`
2. Perform `Self ↦ Dog` substitution: expand `Animal(Dog)` → `{ speak: (self: &Dog) -> String }`
3. Check whether `Dog` provides all required methods (signature match)
4. If passed → generate implementation proof
5. If failed → compile error

**Expansion equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # Expands to Animal's methods, preserving source marker
}

# Equivalent to (preserving source information)
Dog: Type = {
    x: Int,
    speak: (self: &Dog) -> String,  # From Animal, Self replaced by Dog
}
```

**Why source markers are needed**:

- Direct expansion would lose source information
- Source markers are used to generate implementation proofs
- At runtime, the correct method is found through the proof

#### 1.1 Self Type Parameter and Type Checking Timing

`Self` is an explicit type parameter of the interface, not a magic keyword.
`Animal: (Self: Type) -> Type` and `List: (T: Type) -> Type` are the same thing — a `(Type) -> Type`
type constructor.

**Type checking timing**:

- **At interface definition**: `Self` in `{ speak: (self: &Self) -> String }` is an abstract type
  parameter, only syntax-checked.
- **At instantiation point**: When `Animal(Dog)` runs `Self ↦ Dog`, complete type checking
  (signature matching, method existence) is performed after expansion.

This avoids the problem in RFC-011 of `Self` as an implicit magic keyword — `Self` does not appear
in type definitions; it appears only once in the interface parameter list, completely equal to `T`.

#### 1.2 Namespace for Field Names and Method Names

The type's field names and method names share the same namespace. After interface expansion, if an
interface method name conflicts with a type field name, **a compile error is raised**:

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: &Self) -> Int,    # method named x
}

Point: Type = {
    x: Int,                     # field also named x
    Drawable(Point),            # ❌ Compile error: Drawable requires method x, conflicts with field x
}
```

Field access `point.x` and method call `point.x()` are syntactically indistinguishable. A unified
namespace avoids ambiguity.

### 2. Method Implementation

**Core rule**: Both internal and external method declarations are supported.

#### 2.1 Internal Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # Method implementation inside
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# Method implementation outside
Dog.speak: (self: &Dog) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # Some methods inside
}

# Some methods outside
Dog.play: (self: &Dog) -> Void = { ... }
```

**Compiler handling**:

1. Collect all definitions (internal and external)
2. Group by signature (overload)
3. Check for overriding (report error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overload and Override

**Core rules**:

- Different signatures → overload → allowed
- Same signatures → override → report error

#### 3.1 Overload (Allowed)

```yaoxiang
# Different parameter types, overload allowed
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (Forbidden)

```yaoxiang
# Signatures completely identical, override forbidden
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error message**:

```
Error: Dog.speak(self: &Dog) -> String duplicate definition
  --> file2:5:1
  |
5 | Dog.speak: (self: &Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (self: &Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ First definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overload/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# External declaration (overload, allowed)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# External declaration (override, forbidden)
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core rule**: Write `= value` directly after the field, eliminating the need for constructor
functions.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal(Dog),
}
```

**Compiler-generated constructor functions**:

```yaoxiang
# All fields have default values → generate no-arg constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial-arg constructor
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-arg constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External default value declaration**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal(Dog),
}

# External default value declaration
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
    self_param: TypeParam,     // Self type parameter
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internals: type definition
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_instantiations: Vec<InterfaceInstantiation>,
}

// Interface instantiation (Self ↦ ConcreteType)
struct InterfaceInstantiation {
    interface: InterfaceId,
    self_type: TypeId,          // The concrete type Self was replaced with
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
1. Parse type definitions, collect interface instantiation declarations (Animal(Dog))
2. For each interface instantiation, perform Self ↦ ConcreteType substitution
3. Expand interface method signatures, check signature matching
4. Collect all method definitions (internal and external)
5. Group by signature (overload)
6. Check for override (report error)
7. Check interface completeness
8. Generate implementation proof
```

### 6. Dynamic Dispatch

**Core design**: compile-time type collection + interface matching, no vtable.

#### 6.1 Heterogeneous Container

`Animal` is `(Self: Type) -> Type`. `List(Animal)` uses the uninstantiated interface type
constructor as an **existential type**: `∃S. Animal(S)` — "there exists some type S such that S
implements Animal(S)".

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
}

# Heterogeneous container — uninstantiated Animal = existential type
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

**Ownership semantics**: Inserting into a heterogeneous container is Move semantics (RFC-009).
`Dog.new()` is moved into the `AnimalGroup::Dog` enum variant, and the original variable is no
longer available.

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ Compile error: dog has been moved
```

#### 6.2 Compile-Time Type Collection

**Core strategy: ownership tracking, incremental construction.** Not scanning the compile-time for
all types that implement the interface — but rather **incrementally** collecting at each **ownership
operation point** of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → expands to { Dog, Cat }
animals.append(Bird.new())                 // Expands again { Dog, Cat, Bird }
```

**Compiler handling** (incremental):

1. Encounter `List(Animal)` first constructed → generate initial enum (all construction types known
   within the current compilation unit)
2. Each `append` / `push` / index assignment → check whether the value type is already in the enum;
   if not, extend the enum variant
3. Generate monomorphized `match` dispatch code for the final enum
4. Across compilation units: rely on LTO (link-time optimization) to merge enum variants. When
   `Animal` as an existential type is passed across compilation unit boundaries, each unit generates
   partial enum variants, and the link phase merges them into a complete enum.

**Auto-generated enum**:

```yaoxiang
# Compiler auto-generated (user-unaware)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key insight**: Interface matching is a compile-time check, even when the type comes from a
dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: plugin.create_bird()'s return type must implement Animal
bird: Animal = plugin.create_bird()  # compile-time check, existential type

# Insert into heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler handling**:

1. Check the return type of the `append` argument
2. Verify whether that type implements the target interface
3. If passed → extend enum, allow insertion
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call flow (compile-time enum match, ImplementationProof already erased):**

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

**Brand projection** (interaction with RFC-009a): The match pattern binding `AnimalGroup.Dog(d)`
produces a `#animals[0].Dog` sub-brand in the brand tree, equivalent to field projection
(`#42.field_x`). The brand chain `ReadToken(d)` created by `d.speak()` is
`animals → animals[0] → d → ReadToken(d)`, and the borrow checker verifies conflicts through brand
tree prefix matching.

**Type of subscript access**: `animals[0]` returns `&AnimalGroup` (a compiler-generated enum type);
users cannot directly obtain `&mut Animal`. Mutable access is achieved indirectly through interface
methods (e.g., `animals[0].mutate()` internally expands to `AnimalGroup::Dog(d) => d.mutate()`).

**Comparison with vtable**:

|                         | Vtable (Rust)                   | compile-time Enum (YaoXiang)                   |
| ----------------------- | ------------------------------- | ---------------------------------------------- |
| Lookup method           | Vtable pointer → method pointer | Enum match → direct call                       |
| Runtime overhead        | One indirection                 | Branch (optimizable by CPU branch prediction)  |
| compile-time generation | Vtable                          | Enum + match                                   |
| User annotation         | Requires `dyn Trait + 'a`       | Not required                                   |
| ImplementationProof     | Not applicable                  | compile-time erased, does not exist at runtime |

**YaoXiang's advantages**:

- No brand annotation needed
- compile-time type safety
- User-transparent (no need to write `dyn Animal`)
- ImplementationProof is a purely compile-time concept, zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit**: Fully supported. Ownership tracking covers all
`append`/construction points, with incremental enum construction.

**Across compilation units:** Rely on LTO (link-time optimization) to merge enum variants. `Animal`
as an existential type (`∃S. Animal(S)`) is passed across compilation unit boundaries. Each unit
generates partial enum variants, merged at the link phase.

**Not supported**: Runtime dynamic types (full duck typing). The type set is fully known at
compile-time.

#### 6.6 Implementation Notes (Phase 3, v1 landed)

The semantics of §6 (heterogeneous containers, compile-time membership checking, dispatching by
actual type, type set closed at compile-time) have all been implemented, with the concrete form
specified as follows at the mechanism layer:

- **Type collection**: Collects the implementing type set for the entire compilation unit at once
  via `ImplementationProof`, replacing the "incremental collection at each ownership operation
  point" in §6.2. Within a single compilation unit, the two are semantically equivalent (extra dead
  variants are harmless); the value of incremental collection lies in cross-unit scenarios, deferred
  to v2 (see below).
- **Representation**: The compiler synthesizes the `Animal$Group` variant type as pure
  IR/bytecode/runtime artifacts (instructions `CreateVariant`/`VariantTag`/`VariantPayload`, runtime
  value `RuntimeValue::Enum`), MonoType is unaware — the typecheck layer's user-visible type is
  still the interface name. Every concrete value entering an existential type position is
  automatically wrapped as a variant value (uniform opaque representation, §6.4 semantics).
- **Wrap points**: typecheck performs directed walks at positions where "concrete vs. existential"
  judgments occur (annotated let/call argument/return/list literal elements), producing span-keyed
  enforcement tables; IR generation injects wrapping by span. Missed wrapping is loudly rejected by
  runtime guards (`VariantTag`/`VariantPayload` verification that the value must be a named group's
  variant value); the worst case is an explicit runtime error during testing, never silently
  producing incorrect data.
- **Dispatch**: Variant number comparison jump chain, each arm unpacks the payload then statically
  calls the concrete method; the RFC-004 rebinding form (`Type.method = fn[n]`) participates in
  dispatch after rearrangement by binding position.
- **Isolation**: Legacy trait constraints (the `Drawable: Type = {..}` form, no generics parameter)
  do not go through variant dispatch, behavior unchanged.

**v1 boundary (subsequent phases)**: Cross-unit LTO variant merging (§6.5); pattern matching against
Group values (depends on IR support for variant patterns in match); reflection interaction;
Move-into-container semantics; `Any`/type variable transit flows and inferring-lambda boundaries
(fallback = runtime guards).

---

## Use Case Analysis

### Basic Interface Implementation

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# Use
dog = Dog.new()
dog.speak()  # "Woof"
```

### Multiple Interface Implementation

```yaoxiang
# Multiple interfaces
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
}

# Type implementing multiple interfaces
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",
    name: (self: &Dog) -> String = "Buddy",
}

# Use
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### Generic Interface

```yaoxiang
# Generic interface
Container: (Self: Type, T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# Implement generic interface
IntList: Type = {
    data: Array(Int),
    Container(IntList, Int),
    add: (self: &mut IntList, item: Int) -> Void = ...,
    get: (self: &IntList, index: Int) -> Int = ...,
}
```

### Heterogeneous Container

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
}

# Heterogeneous container
animals: List(Animal) = [Dog.new(), Cat.new()]

# Use
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
Plugin: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
    execute: (self: &Self) -> Void,
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
3. **Uniform**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero overhead**: No vtable, compile-time type collection
6. **Type-safe**: Interface matching is a compile-time check
7. **User-transparent**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitation**: Runtime dynamic types (full duck typing) are not supported
2. **compile-time overhead**: Need to generate enum variants and match dispatch code for each
   interface
3. **Type set**: Must be fully known at compile-time (within a single compilation unit)

### Mitigations

1. **Plugin system**: Supported through compile-time interface matching checks
2. **Type set**: Ownership tracking, incremental construction — collected at each
   `append`/construction point, not a global scan
3. **Across compilation units**: Link-time merging of enum variant sets, sharing mechanism with
   link-time monomorphization

---

## Alternatives

| Alternative          | Why not chosen                   |
| -------------------- | -------------------------------- |
| `impl` keyword       | Increases syntax complexity      |
| Vtable (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing     | Runtime overhead, not type-safe  |
| Manual enum wrapping | Heavy user burden                |

---

## Relationship with RFC-009

**Brands and interface implementation**:

- Interface implementation is at the type layer, does not involve brands
- Brands are at the borrow proof layer (RFC-009a)
- The two are orthogonal, mutually independent

**Dynamic dispatch and brands**:

- Dynamic dispatch uses implementation proofs, no brand annotation needed
- Implementation proofs are generated at compile-time, zero lookup at runtime
- Avoids the complexity of `dyn Trait + 'a`

**Ownership of heterogeneous containers**:

- Inserting into `List(Animal)` is Move semantics (RFC-009), the original variable cannot be
  accessed again
- Subscript access `animals[0]` returns `&AnimalGroup` (compiler-generated enum), with brand
  projection chain `animals → animals[0] → enum_variant → field`
- Mutable access is achieved indirectly through interface methods, not exposing `&mut AnimalGroup`
  to the user

## Interface Inheritance

Interfaces can include other interfaces. **No new syntax is introduced** — using the exact same
syntax position as types declaring interfaces:

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet inherits Animal — no new keyword
    name: (self: &Self) -> String,
}

# When Dog implements Pet, it must simultaneously satisfy all methods of Animal and Pet
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",  # From Animal
    name: (self: &Dog) -> String = "Buddy",  # From Pet
}
```

**Design principles:** Inheritance exists but is not encouraged. The primary composition method is
through multiple interface instantiations (`Dog: Type = { Animal(Dog), Pet(Dog), ... }`). A type can
directly declare all interfaces it satisfies, without needing an inheritance tree to express it.
Interface inheritance is used only when there is a clear "is-a" hierarchy.

**Compiler handling**: Expand the inheritance chain. `Pet(Self)` expands to
`{ all methods of Animal(Self), name: ... }`. When `Dog` declares `Pet(Dog)`, `Self ↦ Dog`, and the
compiler verifies that `Dog` simultaneously satisfies all methods of both `Animal(Dog)` and
`Pet(Dog)`.

**Self substitution in interface inheritance**: In
`Pet: (Self: Type) -> Type = { Animal(Self), ... }`, the `Self` in `Animal(Self)` is the `Self`
parameter of `Pet` — it will be lazily substituted. When `Dog` implements `Pet(Dog)`, `Self ↦ Dog`,
and `Animal(Self)` becomes `Animal(Dog)`. This is fully consistent with the parameter passing
semantics of generic functions.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types can choose to
override or inherit the default implementation:

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: &Self) -> String,                      # Must be implemented
    debug: (self: &Self) -> String = self.display(),       # ✅ References same-interface method
    summary: (self: &Self) -> String = f"<{self.name}>",  # ❌ Compile error: self.name not in fmt
}
```

**Core constraint: interfaces cannot assume upper-level implementations.** Default methods can only
reference methods already declared in the same interface. Fields of concrete types or methods of
other interfaces are invisible to default methods — the interface is a closed contract that cannot
reach into the implementing type's pockets. Violations of this constraint are reported as errors
**at interface definition time**.

**Inheritance can assume lower-level implementations:** When interface `Pet(Self)` inherits from
`Animal(Self)`, the default methods of `Pet` can use methods declared by `Animal` — because of
inheritance, they are guaranteed to exist.

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # Inheritance
    name: (self: &Self) -> String,
    introduce: (self: &Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak comes from inherited Animal
}
```

**compile-time behavior**: When a type implements an interface, for each method:

1. Type provides it → use the type's method
2. Type does not provide, interface has default → compiler inlines the default implementation onto
   the type (zero vtable overhead)
3. Type does not provide, interface has no default → compile error

**Design principle:** Default methods are like the auto-derive mechanism of `Copy`/`Clone` — the
compiler auto-generates when needed, and users can override. No `virtual`/`override`/`super`
keywords are introduced.

---

## Implementation Phases

| Phase    | Content                                                                     | Dependency |
| -------- | --------------------------------------------------------------------------- | ---------- |
| Phase 1  | Interface declaration syntax (`(Self: Type) -> Type`) + Self type parameter | RFC-011    |
| Phase 2  | Interface instantiation (`Animal(Dog)`) + Self ↦ ConcreteType substitution  | Phase 1    |
| Phase 3  | Internal/external method declarations                                       | Phase 2    |
| Phase 4  | Overload and override rules                                                 | Phase 3    |
| Phase 5  | Default value syntax                                                        | Phase 3    |
| Phase 6  | Interface inheritance                                                       | Phase 4    |
| Phase 7  | Default method implementation                                               | Phase 6    |
| Phase 8  | Implementation proof generation                                             | Phase 7    |
| Phase 9  | compile-time type collection                                                | Phase 8    |
| Phase 10 | Dynamic dispatch implementation                                             | Phase 9    |

---

## Design Decision Record

| Decision                          | Decision                                                                                                             | Reason                                                                                                                                              | Date       |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Interface declaration syntax      | Interface is a parameterized type `(Self: Type) -> Type`, instantiated at implementation                             | Eliminate the `Self` magic keyword, fully unified with the RFC-011 generics system                                                                  | 2026-06-14 |
| Self type parameter               | Explicit type parameter, only syntax-checked at interface definition, complete check at instantiation point          | Avoid free type variables in HM inference                                                                                                           | 2026-06-14 |
| Dynamic dispatch                  | compile-time type collection + auto enum generation                                                                  | No vtable, zero runtime lookup, user-transparent                                                                                                    | 2026-06-14 |
| External method declaration       | Supported                                                                                                            | Flexibility equivalent to internal declaration, compiler handles cross-file collection                                                              | 2026-06-14 |
| Override                          | Forbidden (same signature reports error)                                                                             | Override leads to unpredictable behavior, overload covers all cases                                                                                 | 2026-06-14 |
| Interface inheritance             | Supported, no new syntax                                                                                             | Same syntax position as type declaring interfaces. Encourages composition (multiple interface instantiations), discourages deep inheritance trees   | 2026-07-03 |
| Default method implementation     | Supported, like Copy/Clone auto-derive                                                                               | Interface provides body, compiler inlines on implementing type; users can override. No virtual/override introduced                                  | 2026-07-03 |
| Default method constraint         | Verify at interface definition: can only reference same-interface methods, cannot assume upper-level implementations | Interface is a closed contract. Inheritance can assume lower-level implementations, but interfaces cannot assume implementing type's fields/methods | 2026-07-03 |
| Type collection strategy          | Ownership tracking, incremental construction — collected at each append/construction point                           | Not a global scan of all implementers, but incremental enum extension at ownership operation points                                                 | 2026-07-03 |
| ImplementationProof               | Purely compile-time concept, erased at runtime                                                                       | Runtime takes enum match dispatch, proof only used for compile-time validation                                                                      | 2026-07-03 |
| Across compilation units          | LTO merges enum variants                                                                                             | Existential type passed across compilation unit boundaries, each unit generates partial enums, merged in LTO phase                                  | 2026-07-03 |
| Field/method namespace            | Unified namespace, conflict reports error                                                                            | Field access `point.x` and method call `point.x()` syntactically indistinguishable, unification avoids ambiguity                                    | 2026-07-03 |
| Heterogeneous container ownership | Move semantics, original variable unusable after insertion                                                           | Consistent with RFC-009 ownership model                                                                                                             | 2026-07-03 |
| Brand projection                  | match pattern binding produces sub-brands, equivalent to field projection                                            | Consistent with RFC-009a brand tree mechanism, enum variant projection is a valid path in the brand tree                                            | 2026-07-03 |
| Receiver spelling convention      | `&Self` borrow / `&mut Self` mutable borrow / by-value = Move                                                        | Receiver follows signature semantics (RFC-009), interface defaults to borrow; historical by-value spelling migrated to &Self                        | 2026-08-30 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → Supported, no new
      syntax. `Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ →
      Supported, like Copy auto-derive. Interface provides body, compiler inlines on demand
- [x] ~~Self as an implicit magic keyword~~ → Eliminated. `Self` is an explicit type parameter, the
      interface is `(Self: Type) -> Type`
- [ ] Advanced usage of interface constraints (associated types, GAT) — associated types are
      implemented via generic interface parameters (`Container: (Self: Type, T: Type) -> Type`), GAT
      requires further design
- [ ] Interaction with closures (closures implementing interfaces) — initial strategy: closures do
      not directly support implementing interfaces, wrapper types are needed. Anonymous type
      interface implementation is left to a subsequent RFC

---

## References

- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Destination

| Status       | Location                    | Description            |
| ------------ | --------------------------- | ---------------------- |
| **accepted** | `docs/design/rfc/accepted/` | Formal design document |
