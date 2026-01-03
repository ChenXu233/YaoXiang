# 标准库实现计划

> 版本：v1.0.0  
> 状态：规划中  
> 日期：2025-01-03

---

## 一、概述

### 1.1 当前状态

标准库目前采用**内联 Rust 函数**的方式实现：

```rust
// src/std/io.rs
pub fn print<T: std::fmt::Display>(value: T) {
    print!("{}", value);
}
```

这种方式：
- ✅ 简单直接，无需 VM 支持
- ✅ 利用 Rust 的 Display trait
- ❌ 不是真正的 YaoXiang 字节码
- ❌ 限制了 YaoXiang 的自举能力

### 1.2 目标

建立真正的 YaoXiang 标准库，实现：
- `print`/`println` - 输出
- `read_line` - 输入
- 数学函数 - abs, sqrt, pow, etc.
- 字符串操作 - length, concat, etc.
- 集合操作 - len, push, pop, etc.

---

## 二、实现策略

### 2.1 策略对比

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      标准库实现策略对比                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  策略 A: 内联 Rust 函数（当前）                                          │
│  ├── 优点：简单、无需 VM 支持                                           │
│  ├── 缺点：不是真正的 YaoXiang 代码                                     │
│  └── 适用：MVP 阶段快速验证                                             │
│                                                                         │
│  策略 B: 字节码内置函数 (Intrinsic)                                      │
│  ├── 优点：真正的字节码，可优化                                         │
│  ├── 缺点：需要 VM 支持 intrinsic 调用                                  │
│  └── 适用：长期方案                                                     │
│                                                                         │
│  策略 C: 外部库链接                                                      │
│  ├── 优点：灵活、可复用 Rust 生态                                       │
│  ├── 缺点：需要链接器支持                                               │
│  └── 适用：复杂功能                                                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 推荐策略：渐进式

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         渐进式实现路径                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Phase 7.1: MVP（当前）                                                 │
│  └── 直接调用 Rust std::fmt::Display                                    │
│                                                                         │
│  Phase 7.2: 字节码内置函数                                               │
│  ├── 添加 StringFromInt/StringFromFloat 指令                           │
│  ├── 添加 StringConcat 指令                                             │
│  └── VM 支持 intrinsic 调用                                            │
│                                                                         │
│  Phase 7.3: 完整的标准库                                                │
│  ├── 所有函数用 YaoXiang 编写                                           │
│  ├── 编译为字节码                                                       │
│  └── 移除对 Rust 标准库的依赖                                           │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 三、MVP 阶段：Rust 函数调用

### 3.1 当前实现方式

标准库函数作为普通 Rust 函数导出：

```rust
// src/std/mod.rs
pub mod io;
pub mod math;
pub mod string;
pub mod list;
pub mod dict;
pub mod concurrent;
pub mod net;

// 使用方式（编译期内联或运行时调用）
```

### 3.2 集成到 VM

VM 通过**外部函数调用**机制调用这些 Rust 函数：

```rust
// src/vm/executor.rs

/// 外部函数调用
fn op_call_external(&mut self, func_name: &str, arg_count: u8) -> Result<(), VMError> {
    // 弹出参数
    let mut args = Vec::new();
    for _ in 0..arg_count {
        args.push(self.stack.pop().unwrap());
    }
    
    // 查找外部函数
    let result = match func_name {
        "print" => self.call_print(args),
        "println" => self.call_println(args),
        "read_line" => self.call_read_line(args),
        _ => return Err(VMError::UnknownExternalFunction(func_name.to_string())),
    };
    
    // 压入结果
    self.stack.push(result);
    Ok(())
}

fn call_print(&mut self, args: Vec<Value>) -> Result<Value, VMError> {
    if let Some(arg) = args.first() {
        print!("{}", arg);
    }
    Ok(Value::Void)
}

fn call_println(&mut self, args: Vec<Value>) -> Result<Value, VMError> {
    if let Some(arg) = args.first() {
        println!("{}", arg);
    }
    Ok(Value::Void)
}
```

### 3.3 外部函数注册表

```rust
// src/vm/external_functions.rs

/// 外部函数签名
type ExternalFunc = fn(Vec<Value>) -> Result<Value, VMError>;

/// 外部函数注册表
pub struct ExternalFunctionTable {
    functions: HashMap<String, ExternalFunc>,
}

impl ExternalFunctionTable {
    pub fn new() -> Self {
        let mut table = ExternalFunctionTable {
            functions: HashMap::new(),
        };
        
        // IO 函数
        table.register("print", |args| {
            if let Some(arg) = args.first() {
                print!("{}", arg);
            }
            Ok(Value::Void)
        });
        
        table.register("println", |args| {
            if let Some(arg) = args.first() {
                println!("{}", arg);
            }
            Ok(Value::Void)
        });
        
        table.register("read_line", |_args| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            Ok(Value::String(input.trim_end().to_string()))
        });
        
        // String 函数
        table.register("string_length", |args| {
            if let Some(Value::String(s)) = args.first() {
                Ok(Value::Int(s.len() as i64))
            } else {
                Err(VMError::TypeMismatch)
            }
        });
        
        // Math 函数
        table.register("math_sqrt", |args| {
            if let Some(Value::Float(f)) = args.first() {
                Ok(Value::Float(f.sqrt()))
            } else {
                Err(VMError::TypeMismatch)
            }
        });
        
        table
    }
    
    pub fn register(&mut self, name: &str, func: ExternalFunc) {
        self.functions.insert(name.to_string(), func);
    }
    
    pub fn call(&self, name: &str, args: Vec<Value>) -> Option<Result<Value, VMError>> {
        self.functions.get(name).map(|f| f(args))
    }
}
```

---

## 四、Phase 7.2: 字节码内置函数

### 4.1 String 操作指令

```rust
// 已有指令（需要 VM 实现）
TypedOpcode::StringLength = 0x90,  // 获取字符串长度
TypedOpcode::StringConcat = 0x91,  // 字符串拼接
TypedOpcode::StringEqual = 0x92,   // 字符串相等比较
TypedOpcode::StringGetChar = 0x93, // 获取字符
TypedOpcode::StringFromInt = 0x94, // 整数转字符串
TypedOpcode::StringFromFloat = 0x95, // 浮点数转字符串
```

### 4.2 指令实现示例

```rust
// src/vm/executor.rs

impl VM {
    fn op_string_length(&mut self, frame: &mut CallFrame) -> Result<(), VMError> {
        let str_reg = frame.stack.pop().unwrap();
        let len = match str_reg {
            Value::String(s) => s.len() as i64,
            _ => return Err(VMError::TypeMismatch),
        };
        frame.stack.push(Value::Int(len));
        Ok(())
    }
    
    fn op_string_concat(&mut self, frame: &mut Self) -> Result<(), VMError> {
        let str2 = frame.stack.pop().unwrap();
        let str1 = frame.stack.pop().unwrap();
        let result = match (str1, str2) {
            (Value::String(s1), Value::String(s2)) => Value::String(s1 + &s2),
            _ => return Err(VMError::TypeMismatch),
        };
        frame.stack.push(result);
        Ok(())
    }
    
    fn op_string_from_int(&mut self, frame: &mut CallFrame) -> Result<(), VMError> {
        let int_val = frame.stack.pop().unwrap();
        let result = match int_val {
            Value::Int(n) => Value::String(n.to_string()),
            _ => return Err(VMError::TypeMismatch),
        };
        frame.stack.push(result);
        Ok(())
    }
}
```

### 4.3 Print 指令

```rust
// src/vm/executor.rs

fn op_print(&mut self, frame: &mut CallFrame) -> Result<(), VMError> {
    let value = frame.stack.pop().unwrap();
    match &value {
        Value::Int(n) => print!("{}", n),
        Value::Float(f) => print!("{}", f),
        Value::String(s) => print!("{}", s),
        Value::Bool(b) => print!("{}", b),
        Value::Char(c) => print!("{}", c),
        Value::Void => print!("void"),
        Value::List(v) => print!("[{:?}]", v),
        _ => print!("{:?}", value),
    }
    Ok(())
}

fn op_println(&mut self, frame: &mut CallFrame) -> Result<(), VMError> {
    self.op_print(frame)?;
    print!("\n");
    Ok(())
}
```

---

## 五、完整标准库列表

### 5.1 IO 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `print(value)` | any | void | Rust intrinsic |
| `println(value)` | any | void | Rust intrinsic |
| `read_line()` | - | String | Rust intrinsic |
| `read_file(path)` | String | String | Rust intrinsic |
| `write_file(path, content)` | String, String | Bool | Rust intrinsic |

### 5.2 String 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `length(s)` | String | Int | StringLength 指令 |
| `concat(s1, s2)` | String, String | String | StringConcat 指令 |
| `equal(s1, s2)` | String, String | Bool | StringEqual 指令 |
| `get_char(s, i)` | String, Int | Char | StringGetChar 指令 |
| `from_int(n)` | Int | String | StringFromInt 指令 |
| `from_float(f)` | Float | String | StringFromFloat 指令 |

### 5.3 Math 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `abs(n)` | Int/Float | Int/Float | I64Abs/F64Abs 指令 |
| `sqrt(n)` | Float | Float | F64Sqrt 指令 |
| `pow(base, exp)` | Float, Float | Float | 指令序列 |
| `sin(n)` | Float | Float | 外部函数 |
| `cos(n)` | Float | Float | 外部函数 |
| `floor(n)` | Float | Float | 外部函数 |
| `ceil(n)` | Float | Float | 外部函数 |

### 5.4 List 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `length(l)` | List | Int | LoadElement + 循环 |
| `push(l, v)` | List, any | List | StoreElement |
| `pop(l)` | List | any | LoadElement |
| `get(l, i)` | List, Int | any | LoadElement |
| `set(l, i, v)` | List, Int, any | void | StoreElement |

### 5.5 Dict 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `new()` | - | Dict | HeapAlloc |
| `set(d, k, v)` | Dict, any, any | void | 指令序列 |
| `get(d, k)` | Dict, any | any | 指令序列 |
| `has(d, k)` | Dict, any | Bool | 指令序列 |
| `remove(d, k)` | Dict, any | void | 指令序列 |

### 5.6 Concurrent 模块

| 函数 | 参数 | 返回 | 实现方式 |
|------|------|------|----------|
| `spawn(f)` | () -> T | Future | Spawn 指令 |
| `await(f)` | Future | T | Await 指令 |
| `sleep(ms)` | Int | void | 外部函数 |
| `channel()` | - | (Sender, Receiver) | 外部函数 |

---

## 六、文件结构

```
src/std/
├── mod.rs              # 标准库入口，导出所有模块
├── io.rs               # IO 函数 (print, read_line, etc.)
├── string.rs           # 字符串函数
├── math.rs             # 数学函数
├── list.rs             # 列表函数
├── dict.rs             # 字典函数
├── concurrent.rs       # 并发函数
└── net.rs              # 网络函数

src/vm/
├── mod.rs
├── executor.rs         # 指令执行（包含 intrinsic 实现）
├── external_functions.rs # 外部函数表
└── intrinsic.rs        # 内置函数实现
```

---

## 七、测试策略

### 7.1 单元测试

```rust
// src/std/io.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_print_int() {
        // 捕获 stdout
        let output = capture_stdout(|| {
            print(42);
        });
        assert_eq!(output, "42");
    }
    
    #[test]
    fn test_println_string() {
        let output = capture_stdout(|| {
            println("hello");
        });
        assert_eq!(output, "hello\n");
    }
    
    #[test]
    fn test_read_line() {
        // Mock stdin
        let mut input = Box::new(std::io::BufReader::new(std::io::Cursor::new("test\n")));
        std::io::stdin = ...;
        
        let result = read_line();
        assert_eq!(result, "test");
    }
}
```

### 7.2 集成测试

```rust
// tests/integration/stdlib.rs

#[test]
fn test_stdlib_print() {
    let code = r#"
        print("Hello, World!")
        println(42)
        print(true)
    "#;
    
    let output = run_and_capture(code);
    assert_eq!(output, "Hello, World!42true\n");
}

#[test]
fn test_stdlib_math() {
    let code = r#"
        print(sqrt(16.0))
    "#;
    
    let output = run_and_capture(code);
    assert!(output.contains("4"));
}

#[test]
fn test_stdlib_string() {
    let code = r#"
        s = "hello"
        print(length(s))
    "#;
    
    let output = run_and_capture(code);
    assert_eq!(output.trim(), "5");
}
```

---

## 八、与 VM 的集成

### 8.1 调用流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         函数调用流程                                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  YaoXiang 代码:                                                         │
│  ─────────────────                                                      │
│  print("hello")                                                        │
│                                                                         │
│  字节码:                                                                │
│  ─────────────────                                                      │
│  LoadConst str_idx                                                     │
│  CallStatic print                                                      │
│                                                                         │
│  VM 执行:                                                               │
│  ─────────────────                                                      │
│  1. 解码 CallStatic 指令                                                │
│  2. 检查函数表                                                          │
│  3. 发现是外部函数                                                      │
│  4. 调用 external_functions.call("print", args)                         │
│  5. 执行 Rust 函数                                                      │
│  6. 返回结果                                                            │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.2 优先级

```
Phase 7.1（当前）：
├── print/println (Rust)
├── read_line (Rust)
└── read_file/write_file (Rust)

Phase 7.2（短期）：
├── StringLength 指令
├── StringConcat 指令
├── StringFromInt/FromFloat 指令
├── Print/Println 指令
└── F64Sqrt 指令

Phase 7.3（中期）：
├── List 函数
├── Dict 函数
├── Math 函数
└── Concurrent 函数
```

---

## 九、验收标准

### 9.1 功能验收

| 函数 | 测试项 | 标准 |
|------|--------|------|
| `print` | Int | 正确输出数字 |
| `print` | Float | 正确输出浮点数 |
| `print` | String | 正确输出字符串 |
| `print` | Bool | 正确输出 true/false |
| `println` | 换行 | 正确输出换行符 |
| `read_line` | 输入 | 正确读取一行 |
| `string_length` | 长度 | 正确返回字符串长度 |
| `string_concat` | 拼接 | 正确拼接两个字符串 |

### 9.2 性能验收

| 测试项 | 标准 |
|--------|------|
| `print` 10万次 | < 1秒 |
| `string_concat` 1万次 | < 100ms |
| `read_line` | < 10ms |

---

## 十、下一步

1. **立即**：使用现有 Rust 实现完成 MVP
2. **短期**：实现 String 操作指令
3. **中期**：实现 Print/Println 指令
4. **长期**：用 YaoXiang 重写所有标准库函数

---

> 「求木之长者，必固其根本；欲流之远者，必浚其泉源。」  
> 标准库是语言的门面，完善的标准库将提升 YaoXiang 的实用性。
