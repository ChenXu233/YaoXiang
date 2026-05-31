```yaml
title: "Data Structure Formatting Rules"
description: "Formatting rules for literals, lists and dictionaries, and Match expressions"
```

# Data Structure Formatting Rules

---

## §8 Literals

**§8.1 Integer literals.** Integer literals are output directly.

```
// ✅ Valid
let x = 42;
```

**§8.2 Floating-point literals.** Floating-point literals must contain a decimal point.

```
// ✅ Valid
let x = 3.14;
let y = 42.0;  // Must have a decimal point

// ❌ Invalid
let y = 42;    // Integer, not floating-point
```

**§8.3 String literals.** Use double quotes by default. Use single quotes when `single_quote = true`.

```
// Default (double quotes)
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 Boolean literals.** Boolean literals use lowercase.

```
// ✅ Valid
let x = true;
let y = false;

// ❌ Invalid
let x = True;
let y = FALSE;
```

---

## §10 Lists and Dictionaries

**§10.1 List format.** Lists are enclosed in `[]`, with elements separated by commas.

```
// ✅ Valid
let x = [1, 2, 3];

// ❌ Invalid
let x = [1,2,3];
```

**§10.2 Dictionary format.** Dictionaries are enclosed in `{}`, with key-value pairs in `key: value` format.

```
// ✅ Valid
let x = {"a": 1, "b": 2};

// ❌ Invalid
let x = {"a":1, "b":2};
```

**§10.3 List comprehension.** List comprehensions use the `[expr for var in iterable]` format.

```
// ✅ Valid
let x = [i * 2 for i in range(10)];

// With condition
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match Expressions

**§11.1 Match format.** Separate the `match` keyword from the expression with a space.

```
// ✅ Valid
match x { ... }

// ❌ Invalid
match(x) { ... }
```

**§11.2 Pattern alignment.** Multiple patterns should be aligned, using spaces for padding.

```
// ✅ Aligned
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern wrapping when too long.** When a pattern is too long, wrap the pattern and align `=>` with the body.

```
// ✅ Wrapped
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```