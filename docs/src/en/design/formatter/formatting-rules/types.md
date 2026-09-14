---
title: 'Type System Formatting Rules'
description: 'Formatting rules for type annotations, references and borrowing, and type conversions'
---

# Type System Formatting Rules

---

## §9 Type Annotations

**§9.1 Variable Type Annotations.** Type annotations use the `: Type` format, with one space after
the colon.

```
// ✅ 正确
let x: Int = 1;

// ❌ 错误
let x:Int = 1;
let x : Int = 1;
```

**§9.2 Function Parameter Types.** The parameter name and type are connected using `: `.

```
// ✅ 正确
fn foo(x: Int, y: String) { ... }

// ❌ 错误
fn foo(x:Int, y:String) { ... }
```

**§9.3 Generic Parameters.** Generic parameters use the `(T: Constraint)` format.

```
// ✅ 正确
fn foo<T: Clone>(x: T) { ... }

// ❌ 错误
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 References and Borrowing

**§15.1 Immutable References.** Use the `&expr` format.

```
// ✅ 正确
let x = &value;

// ❌ 错误
let x = & value;
```

**§15.2 Mutable References.** Use the `&mut expr` format.

```
// ✅ 正确
let x = &mut value;

// ❌ 错误
let x = &mut  value;
let x = & mut value;
```

**§15.3 References in Types.** References in types use the `&Type` or `&mut Type` format.

```
// ✅ 正确
fn foo(x: &Int) { ... }
fn bar(x: &mut Int) { ... }
```

---

## §16 Type Conversion

**§16.1 as Conversion.** Use the `expr as Type` format.

```
// ✅ 正确
let x = value as Int;

// ❌ 错误
let x = value as Int;
let x = value  as  Int;
```

---

## §17 Ref Keyword

**§17.1 Ref Format.** The `ref` keyword is separated from the expression by a space.

```
// ✅ 正确
let x = ref value;
let y = ref obj;

// ❌ 错误
let x = refvalue;  // 缺少空格
let y = ref  value;  // 多余空格
```

**§17.2 Ref Semantics.** `ref` creates an Arc (Atomic Reference Counted) copy.

```
// 创建共享引用
let shared = ref original;
```
