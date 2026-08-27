---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint section of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defines the generics system, but does not detail the interface implementation mechanism.
This document supplements:

1. **Interface declaration**: An interface is a parameterized type — `(Self: Type) -> Type`,
   instantiated with a concrete type at implementation
2. **Method implementation**: Both internal and external declarations are supported
3. **Overload rules**: Different signatures allow overloading; identical signatures error out
   (override prohibited)
4. **Default values**: Write `= value` directly after a field
5. **Dynamic dispatch**: Compile-time type collection + interface matching, no vtable

**Core design**:

```yaoxiang
# Interface definition (parameterized type, Self is an explicit type parameter)
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# Type definition (internal declaration)
Dog: Type = {
    x: Int = 10,
    Animal(Dog),  # Interface instantiation, Self ↦ Dog
    speak: (self: Dog) -> String = "Woof",
}

# External declaration (overload)
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Complexity eliminated**:

- ❌ No `impl` keyword
- ❌ No `Self` magic keyword (`Self` is an explicit type parameter, no different from `T`)
- ❌ No `dyn Trait + 'a` annotation
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (unified overload rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defines the generics system, but does not detail:

| Problem                        | Description                                   |
| ------------------------------ | --------------------------------------------- |
| Interface declaration syntax   | How is interface implementation declared?     |
| Method implementation location | Internal or external declaration?             |
| Overload rules                 | How to handle same-named methods?             |
| Default value syntax           | How to set default values for fields?         |
| Dynamic dispatch               | How are heterogeneous containers implemented? |

### Design Goals

1. **Concise**: No `impl` keyword required
2. **Flexible**: Both internal and external method implementations supported
3. **Unified**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero-cost**: No vtable, compile-time type collection

### Comparison with Rust

| Feature                 | Rust                                   | YaoXiang                            |
| ----------------------- | -------------------------------------- | ----------------------------------- |
| Interface declaration   | `impl Animal for Dog { ... }`          | `Dog: Type = { Animal(Dog), ... }`  |
| Method implementation   | Inside the `impl` block                | Internal or external                |
| Overload                | Not supported                          | Supported (different signatures)    |
| Default value           | Requires `#[default]`                  | Write `= value` directly            |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>`            | `List(Animal)`                      |
| Dynamic dispatch        | Vtable lookup                          | Compile-time type collection        |
| Self keyword            | Magic keyword, implicit quantification | Explicit type parameter, equal to T |

---

## Proposal

### 1. Interface Declaration

**Core rule**: An interface is a parameterized type `(Self: Type) -> Type`, where `Self` is an
explicit type parameter, not a magic keyword. The interface is invoked with a concrete type at
implementation.

```yaoxiang
# Interface definition (completely consistent with RFC-011 generic types)
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# Type declaration implementing the interface
Dog: Type = {
    x: Int,
    Animal(Dog),  # Instantiate the interface, Self ↦ Dog
}
```

**Compiler processing**:

1. Recognize that `Animal(Dog)` is an instantiation of `(Self: Type) -> Type`
2. Perform the `Self ↦ Dog` substitution: expand `Animal(Dog)` → `{ speak: (self: Dog) -> String }`
3. Check whether `Dog` provides all required methods (signature matching)
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
    speak: (self: Dog) -> String,  # From Animal, Self has been substituted with Dog
}
```

**Why source markers are needed**:

- Direct expansion would lose source information
- Source markers are used to generate implementation proof
- At runtime, the correct method is found via the proof

#### 1.1 Self Type Parameter and Type Checking Timing

`Self` is the interface's explicit type parameter, not a magic keyword.
`Animal: (Self: Type) -> Type` and `List: (T: Type) -> Type` are the same thing — a `(Type) -> Type`
type constructor.

**Type checking timing**:

- **At interface definition**: `Self` in `{ speak: (self: Self) -> String }` is an abstract type
  parameter, only syntax-checked.
- **At the instantiation point**: when `Animal(Dog)`, perform `Self ↦ Dog`, and after expansion
  perform full type checking (signature matching, method existence).

This avoids the problem in RFC-011 where `Self` is an implicit magic keyword — `Self` does not
appear in the type definition, it appears only once in the interface parameter list, completely
equal to `T`.

#### 1.2 Field Name and Method Name Namespace

A type's field names and method names share the same namespace. After interface expansion, if the
interface method name conflicts with the type field name, **compile error**:

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: Self) -> Int,    // Method named x
}

Point: Type = {
    x: Int,                     // Field also named x
    Drawable(Point),            // ❌ Compile error: Drawable requires method x, conflicts with field x
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
    speak: (self: Dog) -> String = "Woof",  # Method implementation is internal
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# Method implementation is external
Dog.speak: (self: Dog) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",  # Some methods are internal
}

# Some methods are external
Dog.play: (self: Dog) -> Void = { ... }
```

**Compiler processing**:

1. Collect all definitions (internal and external)
2. Group by signature (overload)
3. Check for overrides (error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overload and Override

**Core rules**:

- Different signatures → overload → allowed
- Identical signatures → override → error

#### 3.1 Overload (allowed)

```yaoxiang
# Different parameter types, overloading is allowed
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (prohibited)

```yaoxiang
# Identical signatures, override is prohibited
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error message**:

```
Error: duplicate definition of Dog.speak(self: Dog) -> String
  --> file2:5:1
  |
5 | Dog.speak: (self: Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (self: Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ first definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overload/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

# External declaration (overload, allowed)
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# External declaration (override, prohibited)
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core rule**: Write `= value` directly after a field, eliminating the need for a constructor.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal(Dog),
}
```

**Compiler-generated constructors**:

```yaoxiang
# All fields have default values → generates a no-arg constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generates partial-parameter constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-parameter constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**Default values via external declaration**:

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
// Compiler internal: interface descriptor
struct InterfaceDescriptor {
    name: String,
    self_param: TypeParam,     // Self type parameter
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internal: type definition
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_instantiations: Vec<InterfaceInstantiation>,
}

// Interface instantiation (Self ↦ ConcreteType)
struct InterfaceInstantiation {
    interface: InterfaceId,
    self_type: TypeId,          // The concrete type Self is substituted with
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
1. Parse type definitions, collect interface instantiation declarations (Animal(Dog))
2. For each interface instantiation, perform Self ↦ ConcreteType substitution
3. Expand interface method signatures, check signature matching
4. Collect all method definitions (internal and external)
5. Group by signature (overload)
6. Check for overrides (error)
7. Check interface completeness
8. Generate implementation proof
```

### 6. Dynamic Dispatch

**Core design**: Compile-time type collection + interface matching, no vtable.

#### 6.1 Heterogeneous Container

`Animal` is `(Self: Type) -> Type`. Using `List(Animal)` with an uninstantiated interface type
constructor as an **existential type**: `∃S. Animal(S)` — "there exists some type S such that S
implements Animal(S)".

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: Cat) -> String = "Meow",
}

# Heterogeneous container — uninstantiated Animal = existential type
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

**Ownership semantics**: Putting something into a heterogeneous container uses Move semantics
(RFC-009). `Dog.new()` is moved into the `AnimalGroup::Dog` enum variant, and the original variable
is no longer usable.

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ Compile error: dog has been moved
```

#### 6.2 Compile-Time Type Collection

**Core strategy: ownership tracking, incremental construction.** Instead of scanning all types
implementing the interface at compile time — incrementally collect at each **ownership operation
point** of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → extends to { Dog, Cat }
animals.append(Bird.new())                 // Further extends to { Dog, Cat, Bird }
```

**Compiler processing** (incremental):

1. Encounter `List(Animal)` being constructed for the first time → generate initial enum (all known
   constructing types within the current compilation unit)
2. At each `append` / `push` / index assignment → check whether the value's type is already in the
   enum; if not, extend the enum variant
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: rely on LTO (Link-Time Optimization) to merge enum variants. When
   `Animal` is passed as an existential type across compilation unit boundaries, each unit generates
   partial enum variants, and the link phase merges them into a complete enum.

**Auto-generated enum**:

```yaoxiang
# Compiler auto-generates (invisible to user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) is internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Match Check

**Key insight**: Interface matching is a compile-time check, even when the type comes from a
dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: the return type of plugin.create_bird() must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check, existential type

# Putting it into a heterogeneous container — the append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) extend the enum
```

**Compiler processing**:

1. Check the return type of the `append` argument
2. Verify whether that type implements the target interface
3. If passed → extend the enum, allow insertion
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call flow (compile-time enum match, ImplementationProof has been erased):**

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

**Brand projection** (interaction with RFC-009a): the pattern binding `AnimalGroup.Dog(d)` in
`match` produces a `#animals[0].Dog` sub-brand in the brand tree, equivalent to field projection
(`#42.field_x`). The `ReadToken(d)` brand chain created by `d.speak()` is
`animals → animals[0] → d → ReadToken(d)`, and the borrow checker validates conflicts via brand tree
prefix matching.

**Type of index access**: `animals[0]` returns `&AnimalGroup` (a compiler-generated enum type), and
the user cannot directly obtain `&mut Animal`. Mutable access is achieved indirectly through
interface methods (e.g., `animals[0].mutate()` internally expands to
`AnimalGroup::Dog(d) => d.mutate()`).

**Comparison with vtable**:

|                         | Vtable (Rust)                   | Compile-time enum (YaoXiang)                      |
| ----------------------- | ------------------------------- | ------------------------------------------------- |
| Lookup method           | Vtable pointer → method pointer | Enum match → direct call                          |
| Runtime overhead        | One indirection                 | Branch (optimizable by CPU branch prediction)     |
| Compile-time generation | Vtable                          | Enum + match                                      |
| User annotation         | Requires `dyn Trait + 'a`       | Not required                                      |
| ImplementationProof     | N/A                             | Erased at compile time, does not exist at runtime |

**YaoXiang's advantages**:

- No brand annotation required
- Compile-time type safety
- User-transparent (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept, with zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit:** Fully supported. Ownership tracking covers all
`append`/construction points, with incremental enum construction.

**Cross-compilation-unit:** Relies on LTO (Link-Time Optimization) to merge enum variants. `Animal`
is passed as an existential type (`∃S. Animal(S)`) across compilation unit boundaries. Each unit
generates partial enum variants, and the link phase merges them.

**Not supported:** Runtime dynamic types (full duck typing). The type set is fully known at compile
time.

---

## Use Case Analysis

### Basic Interface Implementation

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

# Usage
dog = Dog.new()
dog.speak()  # "Woof"
```

### Multiple Interface Implementation

```yaoxiang
# Multiple interfaces
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    name: (self: Self) -> String,
}

# Type implementing multiple interfaces
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: Dog) -> String = "Woof",
    name: (self: Dog) -> String = "Buddy",
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

# Implementing a generic interface
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
    speak: (self: Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: Cat) -> String = "Meow",
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
    name: (self: Self) -> String,
    execute: (self: Self) -> Void,
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

### Pros

1. **Concise**: No `impl` keyword required
2. **Flexible**: Both internal and external method implementations supported
3. **Unified**: Consistent overload rules
4. **Convenient**: Concise default value syntax
5. **Zero-cost**: No vtable, compile-time type collection
6. **Type-safe**: Interface matching is a compile-time check
7. **User-transparent**: No need to write `dyn Animal + 'a`

### Cons

1. **Limitation**: Runtime dynamic types (full duck typing) are not supported
2. **Compile-time overhead**: Requires generating enum variants and match dispatch code for each
   interface
3. **Type set**: Must be fully known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: Supported via compile-time interface match check
2. **Type set**: Ownership tracking, incremental construction — collected at each
   `append`/construction point, not a global scan
3. **Cross-compilation-unit**: Enum variant sets are merged at link time, sharing the mechanism with
   link-time monomorphization

---

## Alternatives

| Alternative          | Why not chosen                   |
| -------------------- | -------------------------------- |
| `impl` keyword       | Adds syntactic complexity        |
| Vtable (`dyn Trait`) | Requires brand annotation (`'a`) |
| Full duck typing     | Runtime overhead, type-unsafe    |
| Manual enum wrapping | Heavy burden on user             |

---

## Relationship with RFC-009

**Brands and interface implementation**:

- Interface implementation lives at the type layer and does not involve brands
- Brands live at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:

- Dynamic dispatch uses the implementation proof, no brand annotation required
- The implementation proof is generated at compile time, with zero runtime lookup
- Avoids the complexity of `dyn Trait + 'a`

**Ownership of heterogeneous containers**:

- Putting something into `List(Animal)` uses Move semantics (RFC-009); the original variable is no
  longer accessible
- Index access `animals[0]` returns `&AnimalGroup` (a compiler-generated enum), with the brand
  projection chain `animals → animals[0] → enum_variant → field`
- Mutable access is achieved indirectly through interface methods, not exposing `&mut AnimalGroup`
  to the user

## Interface Inheritance

An interface can include another interface. **No new syntax is introduced** — the syntax position is
exactly the same as when a type declares an interface:

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet inherits Animal — no new keyword
    name: (self: Self) -> String,
}

# When Dog implements Pet, it must satisfy all methods of both Animal and Pet
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: Dog) -> String = "Woof",  # From Animal
    name: (self: Dog) -> String = "Buddy",  # From Pet
}
```

**Design principle:** Inheritance exists but is not encouraged to be overused. The main composition
approach is through multiple interface instantiations
(`Dog: Type = { Animal(Dog), Pet(Dog), ... }`). A type can directly declare all interfaces it
satisfies, without needing to express this through an inheritance tree. Interface inheritance is
only used when there is a clear "is-a" hierarchy.

**Compiler processing:** Expand the inheritance chain. `Pet(Self)` expands to
`{ all methods of Animal(Self), name: ... }`. When `Dog` declares `Pet(Dog)`, `Self ↦ Dog`, and the
compiler verifies that `Dog` satisfies all methods of both `Animal(Dog)` and `Pet(Dog)`.

**Self substitution in interface inheritance**: In
`Pet: (Self: Type) -> Type = { Animal(Self), ... }`, the `Self` in `Animal(Self)` is `Pet`'s `Self`
parameter — it is delayed-substituted. When `Dog` implements `Pet(Dog)`, `Self ↦ Dog`, and
`Animal(Self)` becomes `Animal(Dog)`. This is completely consistent with the parameter passing
semantics of generic functions.

## Default Method Implementation

An interface can provide default method implementations. The implementing type can choose to
override or inherit the default implementation:

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: Self) -> String,                      # Must be implemented
    debug: (self: Self) -> String = self.display(),       # ✅ References a same-interface method
    summary: (self: Self) -> String = f"<{self.name}>",  # ❌ Compile error: self.name is not in fmt
}
```

**Core constraint: an interface cannot assume superior implementations.** A default method can only
reference methods already declared in the same interface. Fields of the concrete type or methods
from other interfaces are invisible to the default method — the interface is a closed contract that
cannot reach into the implementing type's pocket. Violating this constraint errors out **at
interface definition time**.

**Inheritance can assume subordinate implementations:** When interface `Pet(Self)` inherits from
`Animal(Self)`, the default methods of `Pet` can use methods declared in `Animal` — because of
inheritance, they are guaranteed to exist.

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # Inheritance
    name: (self: Self) -> String,
    introduce: (self: Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak comes from inherited Animal
}
```

**Compile-time behavior:** When a type implements an interface, for each method:

1. The type provides it → use the type's method
2. The type does not provide it, the interface has a default → the compiler inlines the default
   implementation into the type (zero vtable overhead)
3. The type does not provide it, the interface has no default → compile error

**Design principle:** Default methods are similar to the auto-derive mechanism of `Copy`/`Clone` —
the compiler auto-generates when needed, and users can override. The `virtual`/`override`/`super`
keywords are not introduced.

---

## Implementation Phases

| Phase    | Content                                                                     | Dependencies |
| -------- | --------------------------------------------------------------------------- | ------------ |
| Phase 1  | Interface declaration syntax (`(Self: Type) -> Type`) + Self type parameter | RFC-011      |
| Phase 2  | Interface instantiation (`Animal(Dog)`) + Self ↦ ConcreteType substitution  | Phase 1      |
| Phase 3  | Internal/external method implementation                                     | Phase 2      |
| Phase 4  | Overload and override rules                                                 | Phase 3      |
| Phase 5  | Default value syntax                                                        | Phase 3      |
| Phase 6  | Interface inheritance                                                       | Phase 4      |
| Phase 7  | Default method implementation                                               | Phase 6      |
| Phase 8  | Implementation proof generation                                             | Phase 7      |
| Phase 9  | Compile-time type collection                                                | Phase 8      |
| Phase 10 | Dynamic dispatch implementation                                             | Phase 9      |

---

## Design Decision Records

| Decision                          | Decision                                                                                                            | Reason                                                                                                                                                         | Date       |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Interface declaration syntax      | Interface is a parameterized type `(Self: Type) -> Type`, instantiated at implementation                            | Eliminates the `Self` magic keyword, fully consistent with RFC-011 generics system                                                                             | 2026-06-14 |
| Self type parameter               | Explicit type parameter, only syntax-checked at interface definition, fully checked at instantiation                | Avoid free type variables in HM inference                                                                                                                      | 2026-06-14 |
| Dynamic dispatch                  | Compile-time type collection + auto-generated enum                                                                  | No vtable, zero runtime lookup, user-transparent                                                                                                               | 2026-06-14 |
| External method declaration       | Supported                                                                                                           | Equivalent flexibility to internal declaration, compiler handles cross-file collection                                                                         | 2026-06-14 |
| Override                          | Prohibited (same signature errors)                                                                                  | Override leads to unpredictable behavior; overloading covers all cases                                                                                         | 2026-06-14 |
| Interface inheritance             | Supported, no new syntax                                                                                            | Same syntax position as type declaring interface. Encourages composition (multiple interface instantiations), discourages deep inheritance trees               | 2026-07-03 |
| Default method implementation     | Supported, similar to Copy/Clone auto-derive                                                                        | Interface provides default body, compiler inlines on the implementing type; users can override. Does not introduce virtual/override                            | 2026-07-03 |
| Default method constraint         | Verified at interface definition: can only reference same-interface methods, cannot assume superior implementations | The interface is a closed contract. Inheritance can assume subordinate implementations, but an interface cannot assume fields/methods of the implementing type | 2026-07-03 |
| Type collection strategy          | Ownership tracking, incremental construction — collected at each append/construction point                          | Not a global scan of all implementers, but incremental enum extension at ownership operation points                                                            | 2026-07-03 |
| ImplementationProof               | Pure compile-time concept, erased at runtime                                                                        | Runtime takes the enum match path; the proof is only used for compile-time validation                                                                          | 2026-07-03 |
| Cross-compilation-unit            | LTO merges enum variants                                                                                            | Existential types are passed across compilation unit boundaries; each unit generates partial enums, merged in the LTO phase                                    | 2026-07-03 |
| Field/method namespace            | Unified namespace, conflict errors                                                                                  | Field access `point.x` and method call `point.x()` are syntactically indistinguishable; unification avoids ambiguity                                           | 2026-07-03 |
| Heterogeneous container ownership | Move semantics, original variable unusable after insertion                                                          | Consistent with the RFC-009 ownership model                                                                                                                    | 2026-07-03 |
| Brand projection                  | match pattern bindings produce sub-brands, equivalent to field projection                                           | Consistent with the RFC-009a brand tree mechanism; enum variant projection is a valid path in the brand tree                                                   | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit from other interfaces)~~ → Supported, no new
      syntax. `Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ →
      Supported, similar to Copy auto-derive. Interface provides body, compiler inlines on demand
- [x] ~~Self as implicit magic keyword~~ → Eliminated. `Self` is an explicit type parameter, the
      interface is `(Self: Type) -> Type`
- [ ] Advanced usage of interface constraints (associated types, GAT) — associated types are
      implemented via generic interface parameters (`Container: (Self: Type, T: Type) -> Type`), GAT
      requires further design
- [ ] Interaction with closures (closures implementing interfaces) — Initial strategy: closures do
      not directly implement interfaces, a wrapper type is required. Interface implementation for
      anonymous types is left for a future RFC

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
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |
