```markdown
---
title: "spawn"
date: 2025-02-24
tags: ["function"]
weight: 4
---

## spawn

`spawn` creates a new concurrent execution context, commonly used for asynchronous task processing.

### Basic Usage

```yang
spawn task_name() {
    // task body
}
```

### Parameter Description

| Parameter | Type | Description |
|-----------|------|-------------|
| `task_name` | string | The name of the task to be created |
| Body | code block | The execution logic of the task |

### Return Value

Returns a `Promise` representing the execution result of the task.

### Example

```yang
// Create a simple task
let task = spawn my_task() {
    print("Task is running");
    return 42;
};

// Create multiple tasks
let tasks = [
    spawn task_1() { /* ... */ },
    spawn task_2() { /* ... */ },
    spawn task_3() { /* ... */ },
];
```

### Key Points

- `spawn` creates a new **并作** context, which runs concurrently with the main program.
- The task body can contain asynchronous operations.
- Tasks can be managed through the returned `Promise`.

### Best Practices

1. **Error handling**: Always handle potential errors in the task body.
2. **Resource management**: Ensure resources are properly released after task completion.
3. **Concurrency control**: Avoid creating an excessive number of concurrent tasks.
```