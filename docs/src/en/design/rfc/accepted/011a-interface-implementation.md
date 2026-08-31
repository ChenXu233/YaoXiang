---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portion of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defines the generics system, but does not elaborate on the interface implementation
mechanism. This document supplements:

1. **Interface declaration**: An interface is a parameterized type — `(Self: Type) -> Type`, with
   concrete types passed in at implementation
2. **Method implementation**: Both internal and external declarations are supported
3. **Overloading rules**: Overloading allowed when signatures differ; error when signatures are
   identical (override prohibited)
4. **Default values**: Write `= value` directly after the field
5. **Dynamic dispatch**: Compile-time type collection + interface matching, no vtable

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

# External declaration (overloading)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Receiver spelling convention** (errata 2026-08-30, aligned with RFC-009 ownership semantics):

- Method receivers follow signature semantics: `&Self` = borrow (the interface's default convention
  — method calls do not consume the receiver), `&mut Self` = mutable borrow, by-value `Self` =
  consume the receiver (Move, RFC-009).
- `Self` in the impl-side signature is an alias for the impl type: the interface
  `speak: (self: &Self)` matches both impl `(self: &Dog)` and `(self: &Self)` (after Self↦impl type
  substitution, they are fully identical, §3).
- The by-value receiver spelling in historical examples (`(self: Self)`) meant borrow; this document
  has uniformly migrated to the explicit `&Self`; the by-value spelling henceforth retains the
  "consume" semantics and is no longer mixed.

**Eliminated complexity**:

- ❌ No `impl` keyword
- ❌ No `Self` magic keyword (`Self` is an explicit type parameter, no different from `T`)
- ❌ No `dyn Trait + 'a` annotations
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (unified overloading rules)

---

## Motivation

### Insufficiencies of RFC-011

RFC-011 defines the generics system, but does not elaborate on:

| Problem                        | Description                                         |
| ------------------------------ | --------------------------------------------------- |
| Interface declaration syntax   | How to declare that a type implements an interface? |
| Method implementation location | Internal or external declaration?                   |
| Overloading rules              | How to handle methods with the same name?           |
| Default value syntax           | How to set default values for fields?               |
| Dynamic dispatch               | How to implement heterogeneous containers?          |

### Design Goals

1. **Concise**: No `impl` keyword required
2. **Flexible**: Both internal and external method implementation supported
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero overhead**: No vtable, compile-time type collection

### Comparison with Rust

| Feature                 | Rust                                   | YaoXiang                            |
| ----------------------- | -------------------------------------- | ----------------------------------- |
| Interface declaration   | `impl Animal for Dog { ... }`          | `Dog: Type = { Animal(Dog), ... }`  |
| Method implementation   | Inside the `impl` block                | Internal or external                |
| Overloading             | Not supported                          | Supported (different signatures)    |
| Default value           | Requires `#[default]`                  | Write `= value` directly            |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>`            | `List(Animal)`                      |
| Dynamic dispatch        | Vtable lookup                          | Compile-time type collection        |
| Self keyword            | Magic keyword, implicit quantification | Explicit type parameter, equal to T |

---

## Proposal

### 1. Interface Declaration

**Core rule**: An interface is a parameterized type `(Self: Type) -> Type`; `Self` is an explicit
type parameter, not a magic keyword. At implementation, the interface is called with the concrete
type.

```yaoxiang
# Interface definition (fully consistent with RFC-011 generic types)
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type declaration implementing the interface
Dog: Type = {
    x: Int,
    Animal(Dog),  # Instantiate the interface, Self ↦ Dog
}
```

**Compiler processing**:

1. Recognize that `Animal(Dog)` is an instantiation call of `(Self: Type) -> Type`
2. Perform `Self ↦ Dog` substitution: expand `Animal(Dog)` → `{ speak: (self: &Dog) -> String }`
3. Check whether `Dog` provides all required methods (signature matching)
4. If passed → generate implementation proof
5. If failed → compilation error

**Expansion equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # Expanded into Animal's methods, preserving source markers
}

# Equivalent to (with source information preserved)
Dog: Type = {
    x: Int,
    speak: (self: &Dog) -> String,  # From Animal, Self has been replaced with Dog
}
```

**Why source markers are needed**:

- Direct expansion would lose source information
- Source markers are used to generate implementation proof
- At runtime, the correct method is found through the proof

#### 1.1 Self Type Parameter and Type Checking Timing

`Self` is the interface's explicit type parameter, not a magic keyword.
`Animal: (Self: Type) -> Type` and `List: (T: Type) -> Type` are the same kind of thing — a
`(Type) -> Type` type constructor.

**Type checking timing**:

- **At interface definition**: `Self` in `{ speak: (self: &Self) -> String }` is an abstract type
  parameter, only syntactic checking is done.
- **At instantiation point**: When `Animal(Dog)` is encountered, perform `Self ↦ Dog`, and do full
  type checking after expansion (signature matching, method existence).

This avoids the issue of `Self` as an implicit magic keyword in RFC-011 — `Self` does not appear in
type definitions, it only appears once in the interface parameter list, completely equal to `T`.

#### 1.2 Namespace of Field Names and Method Names

A type's field names and method names share the same namespace. After interface expansion, if an
interface method name conflicts with a type field name, **compilation error**:

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: &Self) -> Int,    // method named x
}

Point: Type = {
    x: Int,                     // field also named x
    Drawable(Point),            // ❌ compilation error: Drawable requires method x, conflicts with field x
}
```

Field access `point.x` and method call `point.x()` cannot be distinguished syntactically. A unified
namespace avoids ambiguity.

### 2. Method Implementation

**Core rule**: Both internal and external method implementation declarations are supported.

#### 2.1 Internal Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # Method implementation internal
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# Method implementation external
Dog.speak: (self: &Dog) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # Part of methods internal
}

# Part of methods external
Dog.play: (self: &Dog) -> Void = { ... }
```

**Compiler processing**:

1. Collect all definitions (internal and external)
2. Group by signature (overloading)
3. Check for overrides (report error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Override

**Core rules**:

- Different signatures → overloading → allowed
- Same signature → override → error

#### 3.1 Overloading (Allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (Prohibited)

```yaoxiang
# Identical signatures, override prohibited
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ error: override not allowed
```

**Error message**:

```
Error: duplicate definition Dog.speak(self: &Dog) -> String
  --> file2:5:1
  |
5 | Dog.speak: (self: &Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (self: &Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ first definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overloading/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# External declaration (overloading, allowed)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# External declaration (override, prohibited)
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ error
```

### 4. Default Values

**Core rule**: Write `= value` directly after the field, eliminating the need for a constructor.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # default value
    y: Int = 20,  # default value
    Animal(Dog),
}
```

**Compiler generates constructor**:

```yaoxiang
# All fields have default values → generate parameterless constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial parameter constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full parameter constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External declaration default values**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal(Dog),
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
    self_type: TypeId,          // The concrete type that Self is replaced with
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
2. Perform Self ↦ ConcreteType substitution for each interface instantiation
3. Expand interface method signatures, check signature matching
4. Collect all method definitions (internal and external)
5. Group by signature (overloading)
6. Check for overrides (report error)
7. Check interface completeness
8. Generate implementation proof
```

### 6. Dynamic Dispatch

**Core design**: Compile-time type collection + interface matching, no vtable.

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

**Ownership semantics**: Putting into a heterogeneous container is Move semantics (RFC-009).
`Dog.new()` is moved into the `AnimalGroup::Dog` enum variant; the original variable is no longer
usable.

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ compilation error: dog has been moved
```

#### 6.2 Compile-Time Type Collection

**Core strategy: ownership tracking, incremental construction.** Not scanning all types implementing
the interface at compile time — but incrementally collecting at each **ownership operation point**
of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // compiler sees Cat at the append → extends to { Dog, Cat }
animals.append(Bird.new())                 // further extends { Dog, Cat, Bird }
```

**Compiler processing** (incremental):

1. Encounters `List(Animal)` being constructed for the first time → generate the initial enum (all
   currently known constructed types within the compilation unit)
2. Each `append` / `push` / indexed assignment → check whether the value's type is already in the
   enum; if not, extend the enum variant
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: rely on LTO (link-time optimization) to merge enum variants. When
   `Animal` is passed as an existential type at compilation unit boundaries, each unit generates
   partial enum variants, which are merged into the complete enum at the link stage.

**Auto-generated enum**:

```yaoxiang
# Compiler auto-generated (invisible to the user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) is internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key insight**: Interface matching is checked at compile time, even when the type comes from a
dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: plugin.create_bird() return type must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check, existential type

# Put into heterogeneous container —— append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler processing**:

1. Check the return type of the `append` argument
2. Verify whether that type implements the target interface
3. If passed → extend the enum, allow insertion
4. If failed → compilation error

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
generates a `#animals[0].Dog` sub-brand in the brand tree, equivalent to field projection
(`#42.field_x`). The `ReadToken(d)` brand chain created by `d.speak()` is
`animals → animals[0] → d → ReadToken(d)`, which the borrow checker verifies through brand tree
prefix matching.

**Type of subscript access**: `animals[0]` returns `&AnimalGroup` (the compiler-generated enum
type); the user cannot directly obtain `&mut Animal`. Mutable access is implemented indirectly
through interface methods (e.g., `animals[0].mutate()` internally expands to
`AnimalGroup::Dog(d) => d.mutate()`).

**Comparison with vtable**:

|                         | Vtable (Rust)                   | Compile-time enum (YaoXiang)                       |
| ----------------------- | ------------------------------- | -------------------------------------------------- |
| Lookup method           | Vtable pointer → method pointer | Enum match → direct call                           |
| Runtime overhead        | One level of indirection        | branch (can be optimized by CPU branch prediction) |
| Compile-time generation | Vtable                          | Enum + match                                       |
| User annotation         | Requires `dyn Trait + 'a`       | Not required                                       |
| ImplementationProof     | Not applicable                  | Erased at compile time, does not exist at runtime  |

**YaoXiang's advantages**:

- No brand annotation required
- Compile-time type safety
- User-transparent (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit:** Fully supported. Ownership tracking covers all
`append`/construction points, with incremental enum construction.

**Cross-compilation-unit:** Rely on LTO (link-time optimization) to merge enum variants. `Animal` is
passed as an existential type (`∃S. Animal(S)`) at compilation unit boundaries. Each unit generates
partial enum variants, which are merged at the link stage.

**Not supported:** Runtime dynamic types (full duck typing). The set of types is fully known at
compile time.

#### 6.6 Implementation Notes (#307 Phase 3, v1 landed)

The semantics of §6 (heterogeneous containers, compile-time membership check, dispatch by actual
type, closed type set at compile time) have all landed. The implementation form was concretized at
the mechanism level as follows:

- **Type collection**: Collect the set of implementing types for the entire compilation unit at once
  according to `ImplementationProof`, replacing the "incremental collection at each ownership
  operation point" of §6.2. Within a single compilation unit the two are semantically equivalent
  (extra dead variants are harmless); the value of incremental collection lies in cross-unit
  scenarios, deferred to v2 (see below).
- **Representation**: The compiler synthesizes `Animal$Group` variant types, purely
  IR/bytecode/runtime artifacts (instructions `CreateVariant`/`VariantTag`/`VariantPayload`, runtime
  value `RuntimeValue::Enum`), invisible to MonoType — the user-visible type at the typecheck layer
  is still the interface name. Each concrete value entering an existential type position is
  automatically wrapped as a variant value (unified opaque representation, §6.4 semantics).
- **Wrapping points**: Typecheck does a directed walk at the "concrete vs. existential"
  determination positions (annotated let/call argument/return/list literal element), producing a
  span-keyed mandatory table; IR generation injects wrapping by span. Missed wrapping is loudly
  rejected by runtime guards (`VariantTag`/`VariantPayload` validation that the value must be a
  variant value of the named group); the worst case is an explicit runtime error during testing,
  never silently producing wrong data.
- **Dispatch**: Variant tag comparison branch chain, each arm unpacks the payload then statically
  calls the concrete method; RFC-004 rebinding form (`Type.method = fn[n]`) is rearranged by binding
  position and also participates in dispatch.
- **Isolation**: Legacy trait constraints (`Drawable: Type = {..}` form, no generic parameters) do
  not go through variant dispatch, behavior unchanged.

**v1 boundary (subsequent phases)**: Cross-unit LTO variant merging (§6.5); pattern matching on
Group values (depends on match's IR support for variant patterns); reflection interaction;
Move-into-container semantics; `Any`/type-variable transit flows and inferred-lambda boundaries
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

# Usage
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

# Type implements multiple interfaces
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",
    name: (self: &Dog) -> String = "Buddy",
}

# Usage
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

1. **Concise**: No `impl` keyword required
2. **Flexible**: Both internal and external method implementation supported
3. **Unified**: Consistent overloading rules
4. **Convenient**: Concise default value syntax
5. **Zero overhead**: No vtable, compile-time type collection
6. **Type-safe**: Interface matching is checked at compile time
7. **User-transparent**: No need to write `dyn Animal + 'a`

### Disadvantages

1. **Limited**: Does not support runtime dynamic types (full duck typing)
2. **Compile-time overhead**: Needs to generate enum variants and match dispatch code for each
   interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: Supported through compile-time interface matching checks
2. **Type set**: Ownership tracking, incremental construction — collected at each
   `append`/construction point, not a global scan
3. **Cross-compilation-unit**: Link-time merging of enum variant sets, sharing the mechanism with
   link-time monomorphization

---

## Alternatives

| Plan                   | Why not chosen                   |
| ---------------------- | -------------------------------- |
| `impl` keyword         | Increases syntax complexity      |
| Vtable (`dyn Trait`)   | Requires brand annotation (`'a`) |
| Full duck typing       | Runtime overhead, type-unsafe    |
| Enum wrapping (manual) | Heavy user burden                |

---

## Relationship with RFC-009

**Brand and interface implementation**:

- Interface implementation is at the type layer, not involving brands
- Brands are at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:

- Dynamic dispatch uses implementation proof, no brand annotation required
- Implementation proof is generated at compile time, zero lookup at runtime
- Avoids the complexity of `dyn Trait + 'a`

**Ownership of heterogeneous containers**:

- Putting into `List(Animal)` is Move semantics (RFC-009); the original variable cannot be accessed
  again
- Subscript access `animals[0]` returns `&AnimalGroup` (the compiler-generated enum); the brand
  projection chain is `animals → animals[0] → enum_variant → field`
- Mutable access is implemented indirectly through interface methods, not exposing
  `&mut AnimalGroup` to the user

## Interface Inheritance

An interface can include another interface. **No new syntax is introduced** — it uses exactly the
same syntactic position as type declaration of an interface:

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet inherits Animal — no new keyword
    name: (self: &Self) -> String,
}

# When Dog implements Pet, it must satisfy all methods of both Animal and Pet
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",  # from Animal
    name: (self: &Dog) -> String = "Buddy",  # from Pet
}
```

**Design principle:** Inheritance exists, but its abuse is discouraged. The main composition method
is through multiple interface instantiations (`Dog: Type = { Animal(Dog), Pet(Dog), ... }`). A type
can directly declare all the interfaces it satisfies, without needing to express this through an
inheritance tree. Interface inheritance is only used when there is a clear "is-a" hierarchy.

**Compiler processing:** Expand the inheritance chain. `Pet(Self)` expands to
`{ all methods of Animal(Self), name: ... }`. When `Dog` declares `Pet(Dog)`, `Self ↦ Dog`, and the
compiler verifies that `Dog` satisfies all methods of both `Animal(Dog)` and `Pet(Dog)`.

**Self substitution in interface inheritance**: In
`Pet: (Self: Type) -> Type = { Animal(Self), ... }`, the `Self` in `Animal(Self)` is `Pet`'s `Self`
parameter — it will be lazily substituted. When `Dog` implements `Pet(Dog)`, `Self ↦ Dog`, and
`Animal(Self)` becomes `Animal(Dog)`. This is fully consistent with the parameter passing semantics
of generic functions.

## Default Method Implementation

Interfaces can provide default implementations for methods. The implementing type can choose to
override or inherit the default implementation:

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: &Self) -> String,                      # must be implemented
    debug: (self: &Self) -> String = self.display(),       # ✅ references the same interface method
    summary: (self: &Self) -> String = f"<{self.name}>",  # ❌ compilation error: self.name is not in fmt
}
```

**Core constraint: an interface cannot assume an upper-level implementation.** Default methods can
only reference methods already declared in the same interface. The concrete type's fields or other
interface methods are not visible to default methods — the interface is a closed contract, it cannot
reach into the implementing type's pockets. Violations of this constraint report an error **at the
interface definition**.

**Inheritance can assume a lower-level implementation:** When interface `Pet(Self)` inherits
`Animal(Self)`, `Pet`'s default methods can use methods declared by `Animal` — because it's
inherited, it is guaranteed to exist.

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # inheritance
    name: (self: &Self) -> String,
    introduce: (self: &Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak comes from inherited Animal
}
```

**Compile-time behavior:** When a type implements an interface, for each method:

1. Type provides one → use the type's method
2. Type does not provide, interface has default → compiler inlines the default implementation into
   the type (zero vtable overhead)
3. Type does not provide, interface has no default → compilation error

**Design principle:** Default methods are similar to the auto-derive mechanism of `Copy`/`Clone` —
the compiler auto-generates when needed, and the user can override. No `virtual`/`override`/`super`
keywords are introduced.
---

## Implementation Phases

| Phase    | Content                                                                     | Dependency |
| -------- | --------------------------------------------------------------------------- | ---------- |
| Phase 1  | Interface declaration syntax (`(Self: Type) -> Type`) + Self type parameter | RFC-011    |
| Phase 2  | Interface instantiation (`Animal(Dog)`) + Self ↦ ConcreteType substitution  | Phase 1    |
| Phase 3  | Internal/external declaration of method implementation                      | Phase 2    |
| Phase 4  | Overloading and override rules                                              | Phase 3    |
| Phase 5  | Default value syntax                                                        | Phase 3    |
| Phase 6  | Interface inheritance                                                       | Phase 4    |
| Phase 7  | Default method implementation                                               | Phase 6    |
| Phase 8  | Implementation proof generation                                             | Phase 7    |
| Phase 9  | Compile-time type collection                                                | Phase 8    |
| Phase 10 | Dynamic dispatch implementation                                             | Phase 9    |

---

## Design Decision Records

| Decision                          | Decision                                                                                                          | Reason                                                                                                                                                    | Date       |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Interface declaration syntax      | Interface is a parameterized type `(Self: Type) -> Type`, instantiated at implementation                          | Eliminate the `Self` magic keyword, fully consistent with the RFC-011 generics system                                                                     | 2026-06-14 |
| Self type parameter               | Explicit type parameter, only syntactic check at interface definition, full check at instantiation point          | Avoid free type variables in HM inference                                                                                                                 | 2026-06-14 |
| Dynamic dispatch                  | Compile-time type collection + auto-generated enum                                                                | No vtable, zero runtime lookup, user-transparent                                                                                                          | 2026-06-14 |
| External method declaration       | Supported                                                                                                         | Flexibility equal to internal declaration, compiler is responsible for cross-file collection                                                              | 2026-06-14 |
| Override                          | Prohibited (error on same signature)                                                                              | Override leads to unpredictable behavior, overloading covers all cases                                                                                    | 2026-06-14 |
| Interface inheritance             | Supported, no new syntax                                                                                          | Same syntactic position as type declaration of interface. Encourage composition (multi-interface instantiation), discourage deep inheritance trees        | 2026-07-03 |
| Default method implementation     | Supported, similar to Copy/Clone auto-derive                                                                      | Interface provides default body, compiler inlines on implementing type; user can override. No virtual/override introduced                                 | 2026-07-03 |
| Default method constraint         | Validate at interface definition: only reference same interface methods, cannot assume upper-level implementation | Interface is a closed contract. Inheritance can assume lower-level implementation, but the interface cannot assume the implementing type's fields/methods | 2026-07-03 |
| Type collection strategy          | Ownership tracking, incremental construction — collect at each append/construction point                          | Not a global scan of all implementers, but incremental extension of the enum at ownership operation points                                                | 2026-07-03 |
| ImplementationProof               | Pure compile-time concept, erased at runtime                                                                      | Runtime uses enum match dispatch, proof is only for compile-time verification                                                                             | 2026-07-03 |
| Cross-compilation-unit            | LTO merges enum variants                                                                                          | Existential types are passed at compilation unit boundaries, each unit generates partial enums, merged at the LTO stage                                   | 2026-07-03 |
| Field/method namespace            | Unified namespace, error on conflict                                                                              | Field access `point.x` and method call `point.x()` cannot be distinguished syntactically, unification avoids ambiguity                                    | 2026-07-03 |
| Heterogeneous container ownership | Move semantics, original variable unusable after insertion                                                        | Consistent with RFC-009 ownership model                                                                                                                   | 2026-07-03 |
| Brand projection                  | Match pattern binding generates sub-brand, equivalent to field projection                                         | Consistent with RFC-009a brand tree mechanism, enum variant projection is a valid path in the brand tree                                                  | 2026-07-03 |
| Receiver spelling convention      | `&Self` borrow / `&mut Self` mutable borrow / by-value = Move                                                     | Receiver follows signature semantics (RFC-009), interface default is borrow; historical by-value spelling migrated to &Self                               | 2026-08-30 |

## Open Questions

- [x] ~~Interface inheritance (an interface can inherit other interfaces)~~ → Supported, no new
      syntax. `Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ →
      Supported, similar to Copy auto-derive. Interface provides body, compiler inlines on demand
- [x] ~~Self as an implicit magic keyword~~ → Eliminated. `Self` is an explicit type parameter,
      interface is `(Self: Type) -> Type`
- [ ] Advanced usage of interface constraints (associated types, GAT) — associated types implemented
      through generic interface parameters (`Container: (Self: Type, T: Type) -> Type`), GAT needs
      further design
- [ ] Interaction with closures (closures implementing interfaces) — initial strategy: closures do
      not support directly implementing interfaces, a wrapper type is needed. Interface
      implementation of anonymous types is left to a subsequent RFC

---

## References

- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Destination

| Status       | Location                    | Description              |
| ------------ | --------------------------- | ------------------------ |
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |
