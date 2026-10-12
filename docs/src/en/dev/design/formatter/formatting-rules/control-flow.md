---
title: 'Control Flow Formatting Rules'
description: 'Formatting rules for if/else if/else, for loops, while loops, and loop labels'
---

# Control Flow Formatting Rules

---

## §5 Control Flow

**§5.1 if expression.** The `if` keyword is separated from the condition by a space, and the
condition is separated from the code block by a space.

```
// ✅ Correct
if condition { ... }

// ❌ Incorrect
if(condition) { ... }
if condition{ ... }
```

**§5.2 else if/else.** `else if` and `else` are separated from the preceding code block by a space.

```
// ✅ Correct
if a > 0 { ... } else if a < 0 { ... } else { ... }

// ❌ Incorrect
if a > 0 { ... }else if a < 0 { ... }else { ... }
```

**§5.3 for loop.** The `for` keyword, the variable, the `in` keyword, and the iterator are separated
by spaces.

```
// ✅ Correct
for item in collection { ... }

// ❌ Incorrect
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while loop.** The `while` keyword is separated from the condition by a space.

```
// ✅ Correct
while condition { ... }

// ❌ Incorrect
while(condition) { ... }
```

**§5.5 Loop labels.** The label is connected to the loop keyword by `: `.

```
// ✅ Correct
'outer: for i in range(10) { ... }

// ❌ Incorrect
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```

---

## §5.6 Return Statement

**§5.6.1 Return format.** The `return` keyword is separated from the expression by a space.

```
// ✅ Correct
return 42;
return x + y;

// ❌ Incorrect
return(42);  // Missing space
return  42;  // Extra space
```

**§5.6.2 Empty return.** An empty return uses the `return` keyword directly.

```
// ✅ Correct
return;

// ❌ Incorrect
return ;  // Extra space
return void;  // void is not needed
```

---

## §5.7 Break Statement

**§5.7.1 Break format.** The `break` keyword is separated from the label by a space.

```
// ✅ Correct
break;
break 'outer;

// ❌ Incorrect
break(outer);  // Invalid syntax
break  'outer;  // Extra space
```

---

## §5.8 Continue Statement

**§5.8.1 Continue format.** The `continue` keyword is separated from the label by a space.

```
// ✅ Correct
continue;
continue 'outer;

// ❌ Incorrect
continue(outer);  // Invalid syntax
continue  'outer;  // Extra space
```
