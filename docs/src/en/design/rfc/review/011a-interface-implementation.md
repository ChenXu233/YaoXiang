---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint section of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defines the generics system, but does not elaborate on the interface implementation
mechanism. This document supplements:

1. **Interface declaration**: interfaces are parameterized types — `(Self: Type) -> Type`, with
   concrete types passed in at implementation
2. **Method implementation**: both internal and external declarations are supported
3. **Overloading rules**: overloading is allowed when signatures differ, identical signatures cause
   errors (override is forbidden)
4. **Default values**: write `= value` directly after the field
5. **Dynamic dispatch**: compile-time type collection + interface matching, no vtable

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

# External declaration (overloading)
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Complexity eliminated**:

- ❌ No `impl` keyword
- ❌ No `Self` magic keyword (`Self` is an explicit type parameter, no different from `T`)
- ❌ No `dyn Trait + 'a` annotations
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (unified overloading rules)

---

## Motivation

### Shortcomings of RFC-011

RFC-011 defines the generics system, but does not elaborate on:

| Issue                          | Description                                         |
| ------------------------------ | --------------------------------------------------- |
| Interface declaration syntax   | How to declare that a type implements an interface? |
| Method implementation location | Internal or external declaration?                   |
| Overloading rules              | How are same-named methods handled?                 |
| Default value syntax           | How to set default values for fields?               |
| Dynamic dispatch               | How to implement heterogeneous containers?          |

### Design Goals

1. **Concise**: no `impl` keyword needed
2. **Flexible**: method implementation in both internal or external styles
3. **Unified**: consistent overloading rules
4. **Convenient**: simple default value syntax
5. **Zero overhead**: no vtable, compile-time type collection

### Comparison with Rust

| Feature                 | Rust                                   | YaoXiang                            |
| ----------------------- | -------------------------------------- | ----------------------------------- |
| Interface declaration   | `impl Animal for Dog { ... }`          | `Dog: Type = { Animal(Dog), ... }`  |
| Method implementation   | Inside `impl` block                    | Internal or external                |
| Overloading             | Not supported                          | Supported (different signatures)    |
| Default value           | Requires `#[default]`                  | Write `= value` directly            |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>`            | `List(Animal)`                      |
| Dynamic dispatch        | Vtable lookup                          | Compile-time type collection        |
| Self keyword            | Magic keyword, implicit quantification | Explicit type parameter, equal to T |

---

## Proposal

### 1. Interface Declaration

**Core rule**: An interface is a parameterized type `(Self: Type) -> Type`. `Self` is an explicit
type parameter, not a magic keyword. At implementation, the interface is called and a concrete type
is passed in.

```yaoxiang
# Interface definition (completely consistent with RFC-011 generic types)
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# Type declaration implements the interface
Dog: Type = {
    x: Int,
    Animal(Dog),  # Instantiating the interface, Self ↦ Dog
}
```

**Compiler handling**:

1. Recognize `Animal(Dog)` as an instantiation call of `(Self: Type) -> Type`
2. Perform `Self ↦ Dog` substitution: expand `Animal(Dog)` → `{ speak: (self: Dog) -> String }`
3. Check whether `Dog` provides all required methods (signature matching)
4. If passes → generate implementation proof
5. If fails → compilation error

**Expansion equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # Expands to Animal's methods, preserving source mark
}

# Equivalent to (preserving source information)
Dog: Type = {
    x: Int,
    speak: (self: Dog) -> String,  # Comes from Animal, Self replaced with Dog
}
```

**Why source marks are needed**:

- Direct expansion loses source information
- Source marks are used to generate implementation proof
- The runtime uses the proof to find the correct method

#### 1.1 Self Type Parameter and Type Check Timing

`Self` is the interface's explicit type parameter, not a magic keyword.
`Animal: (Self: Type) -> Type` and `List: (T: Type) -> Type` are the same kind of thing — a
`(Type) -> Type` type constructor.

**Type check timing**:

- **At interface definition**: `Self` in `{ speak: (self: Self) -> String }` is an abstract type
  parameter, only syntax-checked.
- **At instantiation point**: when `Animal(Dog)` is written, `Self ↦ Dog` is performed, and full
  type checking (signature matching, method existence) happens after expansion.

This avoids the problem in RFC-011 where `Self` was an implicit magic keyword — `Self` does not
appear in type definitions, it appears only once in the interface's parameter list, completely equal
to `T`.

#### 1.2 Namespace of Field Names and Method Names

A type's field names and method names share the same namespace. After interface expansion, if an
interface method name conflicts with a type field name, a **compilation error** is raised:

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: Self) -> Int,    // Method named x
}

Point: Type = {
    x: Int,                     // Field also named x
    Drawable(Point),            // ❌ Compilation error: Drawable requires method x, conflicts with field x
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
    speak: (self: Dog) -> String = "Woof",  # Method implementation internal
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# Method implementation external
Dog.speak: (self: Dog) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",  # Some methods internal
}

# Some methods external
Dog.play: (self: Dog) -> Void = { ... }
```

**Compiler handling**:

1. Collect all definitions (internal and external)
2. Group by signature (overloading)
3. Check for override (error if found)
4. Check interface completeness
5. Generate implementation proof

### 3. Overloading and Override

**Core rules**:

- Different signatures → overloading → allowed
- Identical signatures → override → error

#### 3.1 Overloading (Allowed)

```yaoxiang
# Different parameter types, overloading allowed
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (Forbidden)

```yaoxiang
# Completely identical signature, override forbidden
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ Error: override not allowed
```

**Error message**:

```
Error: Dog.speak(self: Dog) -> String redefined
  --> file2:5:1
  |
5 | Dog.speak: (self: Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ redefined
  |
  --> file1:3:1
  |
3 | Dog.speak: (self: Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ first definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overloading/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

# External declaration (overloading, allowed)
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# External declaration (override, forbidden)
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ Error
```

### 4. Default Values

**Core rule**: Write `= value` directly after the field, eliminating the need for a constructor.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # Default value
    y: Int = 20,  # Default value
    Animal(Dog),
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
    self_type: TypeId,          // The concrete type that Self was replaced with
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
5. Group by signature (overloading)
6. Check for override (error if found)
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

**Ownership semantics**: Putting into a heterogeneous container is Move semantics (RFC-009).
`Dog.new()` is moved into the `AnimalGroup::Dog` enum variant, and the original variable is no
longer available.

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ Compilation error: dog has been moved
```

#### 6.2 Compile-Time Type Collection

**Core strategy: ownership tracking, incremental construction.** Not scanning all types implementing
the interface at compile time — but incrementally collecting at each **ownership operation point**
of `List(Animal)`:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // Compiler sees Cat at append → expands to { Dog, Cat }
animals.append(Bird.new())                 // Further expands to { Dog, Cat, Bird }
```

**Compiler handling** (incremental):

1. When `List(Animal)` is first constructed → generate initial enum (all construction types known in
   the current compilation unit)
2. On each `append` / `push` / index assignment → check whether the value's type is already in the
   enum; if not, expand the enum variant
3. Generate monomorphized `match` dispatch code for the final enum
4. Across compilation units: rely on LTO (link-time optimization) to merge enum variants. When
   `Animal` is passed across compilation unit boundaries as an existential type, each unit generates
   partial enum variants, merged into a complete enum at the link stage.

**Auto-generated enum**:

```yaoxiang
# Compiler auto-generated (invisible to user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental expansion
}

# List(Animal) is internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Matching Check

**Key insight**: Interface matching is a compile-time check, even when the type comes from a
dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler check: plugin.create_bird()'s return type must implement Animal
bird: Animal = plugin.create_bird()  # Compile-time check, existential type

# Put into heterogeneous container — append point triggers enum expansion
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # Compiler: (1) verify bird implements Animal (2) expand enum
```

**Compiler handling**:

1. Check the return type of the `append` argument
2. Verify whether the type implements the target interface
3. If passes → expand enum, allow insertion
4. If fails → compilation error

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

**Brand projection** (interaction with RFC-009a): the match pattern binding `AnimalGroup.Dog(d)`
generates the `#animals[0].Dog` sub-brand in the brand tree, equivalent to field projection
(`#42.field_x`). The `ReadToken(d)` brand chain created by `d.speak()` is
`animals → animals[0] → d → ReadToken(d)`, and the borrow checker validates conflicts through brand
tree prefix matching.

**Type of subscript access**: `animals[0]` returns `&AnimalGroup` (the compiler-generated enum
type); the user cannot directly obtain `&mut Animal`. Mutable access is achieved indirectly through
interface methods (e.g., `animals[0].mutate()` internally expands to
`AnimalGroup::Dog(d) => d.mutate()`).

**Comparison with vtable**:

|                        | Vtable (Rust)                   | Compile-time Enum (YaoXiang)                     |
| ---------------------- | ------------------------------- | ------------------------------------------------ |
| Lookup method          | Vtable pointer → method pointer | Enum match → direct call                         |
| Runtime overhead       | One level of indirection        | branch (optimizable by CPU branch prediction)    |
| Compile-time generated | Vtable                          | Enum + match                                     |
| User annotations       | Requires `dyn Trait + 'a`       | Not required                                     |
| ImplementationProof    | Not applicable                  | Erased at compile time, doesn't exist at runtime |

**YaoXiang's advantages**:

- No brand annotations needed
- Compile-time type safety
- User-transparent (no need to write `dyn Animal`)
- ImplementationProof is a pure compile-time concept with zero runtime overhead

#### 6.5 Limitations and Scope

**Single compilation unit (current phase):** Fully supported. Ownership tracking covers all
`append`/construction points, and enum is built incrementally.

**Cross compilation unit:** Relies on LTO (link-time optimization) to merge enum variants. `Animal`
is passed across compilation unit boundaries as an existential type (`∃S. Animal(S)`). Each unit
generates partial enum variants, merged at the link stage.

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

# Type implements multiple interfaces
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

# Implementing the generic interface
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

### Advantages

1. **Concise**: no `impl` keyword needed
2. **Flexible**: method implementation in both internal or external styles
3. **Unified**: consistent overloading rules
4. **Convenient**: simple default value syntax
5. **Zero overhead**: no vtable, compile-time type collection
6. **Type-safe**: interface matching is a compile-time check
7. **User-transparent**: no need to write `dyn Animal + 'a`

### Disadvantages

1. **Limitation**: no runtime dynamic types (full duck typing)
2. **Compile-time overhead**: enum variants and match dispatch code must be generated for each
   interface
3. **Type set**: must be fully known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: supported through compile-time interface matching check
2. **Type set**: ownership tracking, incremental construction — collected at each
   `append`/construction point, not globally scanned
3. **Cross compilation unit**: link-time merging of enum variant sets, sharing the mechanism with
   link-time monomorphization

---

## Alternatives

| Approach               | Why not chosen                    |
| ---------------------- | --------------------------------- |
| `impl` keyword         | Increases syntax complexity       |
| Vtable (`dyn Trait`)   | Requires brand annotations (`'a`) |
| Full duck typing       | Runtime overhead, type-unsafe     |
| Enum wrapping (manual) | Heavy user burden                 |

---

## Relationship with RFC-009

**Brands and interface implementation**:

- Interface implementation is at the type layer, not involving brands
- Brands are at the borrow proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:

- Dynamic dispatch uses implementation proof, no brand annotations needed
- Implementation proof is compile-time generated, zero lookup at runtime
- Avoids the complexity of `dyn Trait + 'a`

**Ownership of heterogeneous containers**:

- Putting into `List(Animal)` is Move semantics (RFC-009), the original variable cannot be accessed
  again
- Subscript access `animals[0]` returns `&AnimalGroup` (compiler-generated enum), the brand
  projection chain is `animals → animals[0] → enum_variant → field`
- Mutable access is achieved indirectly through interface methods, without exposing
  `&mut AnimalGroup` to the user

## Interface Inheritance

Interfaces can include other interfaces. **No new syntax introduced** — uses exactly the same syntax
position as types declaring interfaces:

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

**Design principle:** Inheritance exists, but its abuse is discouraged. The main composition method
is through multiple interface instantiations (`Dog: Type = { Animal(Dog), Pet(Dog), ... }`). A type
can directly declare all interfaces it satisfies, without needing an inheritance tree to express.
Interface inheritance is used only when there is a clear "is-a" hierarchy.

**Compiler handling:** Expand the inheritance chain. `Pet(Self)` expands to
`{ all methods from Animal(Self), name: ... }`. When `Dog` declares `Pet(Dog)`, `Self ↦ Dog`, and
the compiler verifies that `Dog` satisfies all methods of both `Animal(Dog)` and `Pet(Dog)`.

**Self substitution in interface inheritance**: in
`Pet: (Self: Type) -> Type = { Animal(Self), ... }`, the `Self` in `Animal(Self)` is `Pet`'s `Self`
parameter — it gets substituted lazily. When `Dog` implements `Pet(Dog)`, `Self ↦ Dog`, and
`Animal(Self)` becomes `Animal(Dog)`. This is completely consistent with the parameter passing
semantics of generic functions.

## Default Method Implementation

Interfaces can provide default implementations for methods. Implementing types can choose to
override or inherit the default implementation:

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: Self) -> String,                      # Must implement
    debug: (self: Self) -> String = self.display(),       # ✅ References same-interface method
    summary: (self: Self) -> String = f"<{self.name}>",  # ❌ Compilation error: self.name is not in fmt
}
```

**Core constraint: interfaces cannot assume super-level implementations.** Default methods can only
reference methods already declared in the same interface. Fields of concrete types or methods of
other interfaces are invisible to default methods — an interface is a closed contract, it cannot
reach into the implementing type's pockets. Violating this constraint is reported as an error **at
interface definition time**.

**Inheritance can assume sub-level implementations:** when interface `Pet(Self)` inherits
`Animal(Self)`, `Pet`'s default methods can use methods declared by `Animal` — because they are
inherited, so they are guaranteed to exist.

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

**Compile-time behavior:** when a type implements an interface, for each method:

1. Type provides one → use the type's method
2. Type doesn't provide, interface has default → compiler inlines the default implementation into
   the type (zero vtable overhead)
3. Type doesn't provide, interface has no default → compilation error

**Design principle:** Default methods are similar to the auto-derive mechanism of `Copy`/`Clone` —
the compiler auto-generates when needed, and the user can override. No `virtual`/`override`/`super`
keywords introduced.
---

## Implementation Phases

| Phase    | Content                                                                     | Dependencies |
| -------- | --------------------------------------------------------------------------- | ------------ |
| Phase 1  | Interface declaration syntax (`(Self: Type) -> Type`) + Self type parameter | RFC-011      |
| Phase 2  | Interface instantiation (`Animal(Dog)`) + Self ↦ ConcreteType substitution  | Phase 1      |
| Phase 3  | Internal/external method declaration                                        | Phase 2      |
| Phase 4  | Overloading and override rules                                              | Phase 3      |
| Phase 5  | Default value syntax                                                        | Phase 3      |
| Phase 6  | Interface inheritance                                                       | Phase 4      |
| Phase 7  | Default method implementation                                               | Phase 6      |
| Phase 8  | Implementation proof generation                                             | Phase 7      |
| Phase 9  | Compile-time type collection                                                | Phase 8      |
| Phase 10 | Dynamic dispatch implementation                                             | Phase 9      |

---

## Design Decision Records

| Decision                          | Decision                                                                                                                | Reason                                                                                                                                            | Date       |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Interface declaration syntax      | Interface is parameterized type `(Self: Type) -> Type`, instantiated at implementation                                  | Eliminates the `Self` magic keyword, fully unified with RFC-011 generics system                                                                   | 2026-06-14 |
| Self type parameter               | Explicit type parameter, syntax-only check at interface definition, full check at instantiation                         | Avoids free type variables in HM inference                                                                                                        | 2026-06-14 |
| Dynamic dispatch                  | Compile-time type collection + auto enum generation                                                                     | No vtable, zero runtime lookup, user-transparent                                                                                                  | 2026-06-14 |
| External method declaration       | Supported                                                                                                               | Flexibility equivalent to internal declaration, compiler handles cross-file collection                                                            | 2026-06-14 |
| Override                          | Forbidden (error on same signature)                                                                                     | Override causes unpredictable behavior, overloading covers all cases                                                                              | 2026-06-14 |
| Interface inheritance             | Supported, no new syntax                                                                                                | Same syntax position as type-declared interfaces. Encourage composition (multiple interface instantiation), discourage deep inheritance trees     | 2026-07-03 |
| Default method implementation     | Supported, similar to Copy/Clone auto-derive                                                                            | Interface provides default body, compiler inlines onto implementing type; user can override. No `virtual`/`override` introduced                   | 2026-07-03 |
| Default method constraint         | Validated at interface definition: can only reference same-interface methods, cannot assume super-level implementations | Interface is a closed contract. Inheritance can assume sub-level implementations, but interfaces cannot assume implementing type's fields/methods | 2026-07-03 |
| Type collection strategy          | Ownership tracking, incremental construction — collected at each append/construction point                              | Not a global scan of all implementers, but incremental enum expansion at each ownership operation point                                           | 2026-07-03 |
| ImplementationProof               | Pure compile-time concept, erased at runtime                                                                            | Runtime uses enum match dispatch, proof is only used for compile-time validation                                                                  | 2026-07-03 |
| Cross compilation unit            | LTO merges enum variants                                                                                                | Existential type is passed across compilation unit boundaries, each unit generates partial enum, LTO merges                                       | 2026-07-03 |
| Field/method namespace            | Unified namespace, conflict error                                                                                       | Field access `point.x` and method call `point.x()` are syntactically indistinguishable, unification avoids ambiguity                              | 2026-07-03 |
| Heterogeneous container ownership | Move semantics, original variable unusable after insertion                                                              | Consistent with RFC-009 ownership model                                                                                                           | 2026-07-03 |
| Brand projection                  | Match pattern binding produces sub-brands, equivalent to field projection                                               | Consistent with RFC-009a brand tree mechanism, enum variant projection is a valid path in the brand tree                                          | 2026-07-03 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → Supported, no new
      syntax. `Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ →
      Supported, similar to Copy auto-derive. Interface provides body, compiler inlines on demand
- [x] ~~Self as implicit magic keyword~~ → Eliminated. `Self` is an explicit type parameter,
      interface is `(Self: Type) -> Type`
- [ ] Advanced use of interface constraints (associated types, GAT) — associated types implemented
      via generic interface parameters (`Container: (Self: Type, T: Type) -> Type`), GAT requires
      further design
- [ ] Interaction with closures (closures implementing interfaces) — initial strategy: closures do
      not support direct interface implementation, wrapper types are needed. Anonymous type
      interface implementation left for a future RFC

---

## References

- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md) — Parent RFC
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Ownership system
- [RFC-009a: Borrow Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md) — Brand mechanism
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Unified syntax

---

## Lifecycle and Destination

| Status           | Location                  | Description               |
| ---------------- | ------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
