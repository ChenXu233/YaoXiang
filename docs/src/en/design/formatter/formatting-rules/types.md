---
title: "Type System Formatting Rules"
description: Formatting rules for type annotations, references and borrowing, type conversion
---

# Type System Formatting Rules

---

## §9 Type Annotations

**§9.1 Variable type annotations.** Type annotations use the `: Type` format, with one space after the colon.

```
// ✅ Correct
let x: Int = 1;

// ❌ Incorrect
let x:Int = 1;
let x : Int = 1;
```

**§9.2 Function parameter types.** Parameter names and types are connected using `: `.

```
// ✅ Correct
fn foo(x: Int, y: String) { ... }

// ❌ Incorrect
fn foo(x:Int, y:String) { ... }
```

**§9.3 Generic parameters.** Generic parameters use the `(T: Constraint)` format.

```
// ✅ Correct
fn foo<T: Clone>(x: T) { ... }

// ❌ Incorrect
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 References and Borrowing

**§15.1 Immutable references.** Use the `&expr` format.

```
// ✅ Correct
let x = &value;

// ❌ Incorrect
let x = & value;
```

**§15.2 Mutable references.** Use the `&mut expr` format.

```
// ✅ Correct
let x = &mut value;

// ❌ Incorrect
let x = &mut  value;
let x = & mut value;
```

**§15.3 References in types.** References in types use the `&Type` or `&mut Type` format.

```
// ✅ Correct
fn foo(x: &Int) { ... }
fn bar(x: &mut Int) { ... }
```

---

## §16 Type Conversion

**§16.1 as conversion.** Use the `expr as Type` format.

```
// ✅ Correct
let x = value as Int;

// ❌ Incorrect
let x = value as Int;
let x = value  as  Int;
```

---

## §17 The ref Keyword

**§17.1 ref format.** The `ref` keyword is separated from the expression by a space.

```
// ✅ Correct
let x = ref value;
let y = ref obj;

// ❌ Incorrect
let x = refvalue;  // Missing space
let y = ref  value;  // Extra space
```

**§17.2 ref semantics.** `ref` creates an Arc (atomic reference counting) copy.

```
// Create a shared reference
let shared = ref original;
```