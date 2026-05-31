---
title: "Control Flow Formatting Rules"
description: "Formatting rules for if/elif/else, for loops, while loops, and loop labels"
---

# Control Flow Formatting Rules

---

## §5 Control Flow

**§5.1 if expressions.** Separate the `if` keyword from the condition with a space, and the condition from the code block with a space.

```
// ✅ Correct
if condition { ... }

// ❌ Incorrect
if(condition) { ... }
if condition{ ... }
```

**§5.2 elif/else.** Separate `elif` and `else` from the preceding code block with a space.

```
// ✅ Correct
if a > 0 { ... } elif a < 0 { ... } else { ... }

// ❌ Incorrect
if a > 0 { ... }elif a < 0 { ... }else { ... }
```

**§5.3 for loops.** Separate the `for` keyword, variable, `in` keyword, and iterator with spaces.

```
// ✅ Correct
for item in collection { ... }

// ❌ Incorrect
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while loops.** Separate the `while` keyword from the condition with a space.

```
// ✅ Correct
while condition { ... }

// ❌ Incorrect
while(condition) { ... }
```

**§5.5 Loop labels.** Connect the label and the loop keyword with `: `.

```
// ✅ Correct
'outer: for i in range(10) { ... }

// ❌ Incorrect
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```