---
title: 'Control Flow Formatting Rules'
description: 'Formatting rules for if/else if/else, for loops, while loops, and loop labels'
---

# Control Flow Formatting Rules

---

## §5 Control Flow

**§5.1 if Expression.** A space separates the `if` keyword from the condition, and a space separates
the condition from the code block.

```
// ✅ Correct
if condition { ... }

// ❌ Incorrect
if(condition) { ... }
if condition{ ... }
```

**§5.2 else if/else.** A space separates `else if` and `else` from the previous code block.

```
// ✅ Correct
if a > 0 { ... } else if a < 0 { ... } else { ... }

// ❌ Incorrect
if a > 0 { ... }else if a < 0 { ... }else { ... }
```

**§5.3 for Loop.** A space separates the `for` keyword, variable, `in` keyword, and iterator.

```
// ✅ Correct
for item in collection { ... }

// ❌ Incorrect
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while Loop.** A space separates the `while` keyword from the condition.

```
// ✅ Correct
while condition { ... }

// ❌ Incorrect
while(condition) { ... }
```

**§5.5 Loop Labels.** The label connects to the loop keyword with `: `.

```
// ✅ Correct
'outer: for i in range(10) { ... }

// ❌ Incorrect
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```

---

## §5.6 Return Statements

**§5.6.1 Return Format.** A space separates the `return` keyword from the expression.

```
// ✅ Correct
return 42;
return x + y;

// ❌ Incorrect
return(42);  // Missing space
return  42;  // Extra space
```

**§5.6.2 Empty Return.** An empty return uses the `return` keyword directly.

```
// ✅ Correct
return;

// ❌ Incorrect
return ;  // Extra space
return void;  // void not needed
```

---

## §5.7 Break Statements

**§5.7.1 Break Format.** A space separates the `break` keyword from the label.

```
// ✅ Correct
break;
break 'outer;

// ❌ Incorrect
break(outer);  // Wrong syntax
break  'outer;  // Extra space
```

---

## §5.8 Continue Statements

**§5.8.1 Continue Format.** A space separates the `continue` keyword from the label.

```
// ✅ Correct
continue;
continue 'outer;

// ❌ Incorrect
continue(outer);  // Wrong syntax
continue  'outer;  // Extra space
```
