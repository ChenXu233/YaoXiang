---
title: 'Data Structure Formatting Rules'
description: 'Formatting rules for literals, lists and dictionaries, Match expressions'
---

# Data Structure Formatting Rules

---

## §8 Literals

**§8.1 Integer literals.** Integer literals are output directly.

```
// ✅ Correct
let x = 42;
```

**§8.2 Float literals.** Float literals must include a decimal point.

```
// ✅ Correct
let x = 3.14;
let y = 42.0;  // Must have decimal point

// ❌ Wrong
let y = 42;    // Integer, not float
```

**§8.3 String literals.** Double quotes are used by default. When `single_quote = true`, single quotes are used.

```
// Default (double quotes)
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 Boolean literals.** Boolean literals use lowercase.

```
// ✅ Correct
let x = true;
let y = false;

// ❌ Wrong
let x = True;
let y = FALSE;
```

---

## §10 Lists and Dictionaries

**§10.1 List formatting.** Lists are enclosed in `[]`, with elements separated by commas.

```
// ✅ Correct
let x = [1, 2, 3];

// ❌ Wrong
let x = [1,2,3];
```

**§10.2 Dictionary formatting.** Dictionaries are enclosed in `{}`, with key-value pairs in `key: value` format.

```
// ✅ Correct
let x = {"a": 1, "b": 2};

// ❌ Wrong
let x = {"a":1, "b":2};
```

**§10.3 List comprehension.** List comprehension uses `[expr for var in iterable]` format.

```
// ✅ Correct
let x = [i * 2 for i in range(10)];

// With condition
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match Expressions

**§11.1 Match formatting.** The `match` keyword is separated from the expression by a space.

```
// ✅ Correct
match x { ... }

// ❌ Wrong
match(x) { ... }
```

**§11.2 Pattern alignment.** Multiple patterns should be aligned, with spaces used for padding.

```
// ✅ Aligned
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern wrapping when too long.** When a pattern is too long, the pattern wraps to a new line, with `=>` aligned with the body.

```
// ✅ Wrapped
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```

---

## §11.4 Tuples

**§11.4.1 Tuple formatting.** Tuples are enclosed in `()`, with elements separated by commas.

```
// ✅ Correct
let t = (1, "hello", true);
let t = (1,);  // Single-element tuple

// ❌ Wrong
let t = (1, "hello", true);  // Missing space after comma
let t = (1,"hello",true);  // Missing space after comma
```

**§11.4.2 Empty tuple.** Empty tuple is represented by `()`.

```
// ✅ Correct
let t = ();
```

---

## §11.5 Index Access

**§11.5.1 Index formatting.** Index access uses `expr[index]` format.

```
// ✅ Correct
let x = arr[0];
let y = matrix[i][j];

// ❌ Wrong
let x = arr [0];  // Extra space
let y = matrix[ i ][ j ];  // Extra spaces
```

---

## §11.6 Field Access

**§11.6.1 Field access formatting.** Field access uses `expr.field` format.

```
// ✅ Correct
let x = obj.field;
let y = obj.method();

// ❌ Wrong
let x = obj . field;  // Extra space
let y = obj. field;  // Extra space
```

**§11.6.2 Chained field access.** When chained field access exceeds the line width, each method call goes on its own line.

```
// Exceeds line width
let result = object.method1().method2().method3().method4();

// After formatting
let result = object
    .method1()
    .method2()
    .method3()
    .method4();
```
