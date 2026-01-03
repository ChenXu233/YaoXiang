# Phase 6: 字节码虚拟机 (VM) 实现计划

> 版本：v1.0.0  
> 状态：规划中  
> 日期：2025-01-03

---

## 一、当前状态评估

### 1.1 代码生成器现状

| 模块 | 状态 | 完成度 | 说明 |
|------|------|--------|------|
| `TypedOpcode` | ✅ 完成 | 100% | 约90条指令定义完整 |
| `bytecode.rs` | ✅ 完成 | 90% | 序列化/反序列化框架 |
| `expr.rs` | ⚠️ 部分 | 70% | 表达式生成基本可用 |
| `stmt.rs` | ⚠️ 简化 | 40% | 语句生成简化处理 |
| `control_flow.rs` | ⚠️ 部分 | 50% | 控制流框架存在 |

### 1.2 已知问题

1. **控制流缺失**：if/while/for/match 生成缺少 `Jmp`/`JmpIf` 指令
2. **语句简化**：`generate_stmt` 返回空实现
3. **返回处理**：`ReturnValue` 未在表达式中正确使用
4. **函数调用**：参数传递方式可能需要调整

---

## 二、MVP 目标

### 2.1 最小可行目标

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MVP 功能范围                                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  能够运行简单的 YaoXiang 程序：                                          │
│                                                                         │
│  • 变量声明与赋值                                                       │
│  • 算术运算 (+ - * /)                                                   │
│  • 布尔运算与比较                                                       │
│  • 条件分支 (if-else)                                                   │
│  • 循环 (while)                                                         │
│  • 函数定义与调用                                                       │
│  • 基本类型 (Int, Float, Bool, String)                                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 成功标准

- [ ] 能编译并运行 hello.yx
- [ ] 能执行算术表达式
- [ ] 能执行条件分支
- [ ] 能执行循环
- [ ] 能调用函数

---

## 三、实施策略

### 3.1 策略：分阶段实现

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           实现阶段                                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Phase 6.1: 完善代码生成器 (1-2 天)                                      │
│  ├── 修复控制流生成 (添加跳转指令)                                       │
│  ├── 完善语句生成 (变量声明、返回语句)                                   │
│  └── 添加基本测试用例                                                   │
│                                                                         │
│  Phase 6.2: VM 核心框架 (1-2 天)                                        │
│  ├── VM 运行时结构                                                      │
│  ├── 指令解码与执行循环                                                 │
│  └── 调用帧管理                                                         │
│                                                                         │
│  Phase 6.3: 核心指令实现 (2-3 天)                                       │
│  ├── 栈与寄存器操作                                                     │
│  ├── 算术与比较运算                                                     │
│  ├── 控制流指令                                                         │
│  └── 函数调用与返回                                                     │
│                                                                         │
│  Phase 6.4: 测试与调优 (1-2 天)                                         │
│  ├── 单元测试                                                           │
│  ├── 集成测试                                                           │
│  └── 性能调优                                                           │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 四、详细实施步骤

### Phase 6.1: 完善代码生成器

#### 任务 1.1: 修复控制流生成

**文件**: `src/middle/codegen/control_flow.rs`

```rust
// 需要实现的 If 生成的伪代码
fn generate_if(&mut self, condition: &Expr, then_branch: &Block, else_branch: Option<&Block>) {
    let end_label = self.next_label();
    let else_label = self.next_label();

    // 生成条件
    let cond = self.generate_expr(condition)?;

    // 条件为假跳转到 else
    self.emit(JmpIfNot(cond, else_label));

    // 生成 then 分支
    self.enter_block(then_label);
    self.generate_block(then_branch)?;
    self.emit(Jmp(end_label));

    // 生成 else 分支
    self.enter_block(else_label);
    if let Some(else_b) = else_branch {
        self.generate_block(else_b)?;
    }
    self.emit(Jmp(end_label));

    // 结束标签
    self.enter_block(end_label);
}
```

**验收标准**：
- [ ] `if` 语句生成 `JmpIfNot` 指令
- [ ] `if-else` 语句生成正确的跳转链
- [ ] 有测试用例覆盖

#### 任务 1.2: 完善语句生成

**文件**: `src/middle/codegen/stmt.rs`

需要实现的方法：
- `generate_var_decl` - 变量声明
- `generate_return_stmt` - 返回语句
- `generate_fn_def` - 函数定义

#### 任务 1.3: 修复返回处理

**问题**：
- 表达式中的 `return` 返回错误
- 函数返回值未正确使用 `ReturnValue`

**修复**：
```rust
// Expr::Return 应该返回 Operand
Expr::Return(value, _) => {
    if let Some(v) = value {
        let operand = self.generate_expr(v)?;
        self.emit(BytecodeInstruction::new(
            TypedOpcode::ReturnValue,
            vec![self.operand_to_reg(&operand)?],
        ));
    } else {
        self.emit(BytecodeInstruction::new(TypedOpcode::Return, vec![]));
    }
    Ok(Operand::Temp(self.next_temp()))
}
```

---

### Phase 6.2: VM 核心框架

#### 任务 2.1: VM 运行时结构

**文件**: `src/vm/mod.rs`

```rust
/// 虚拟机
pub struct VM {
    /// 常量池
    const_pool: Vec<ConstValue>,
    
    /// 类型表
    type_table: Vec<MonoType>,
    
    /// 调用栈
    call_stack: Vec<CallFrame>,
    
    /// 全局变量
    globals: Vec<Value>,
    
    /// 指令指针
    ip: usize,
    
    /// 运行时配置
    config: VMConfig,
}

/// 运行时配置
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// 是否启用调试模式
    debug_mode: bool,
    
    /// 最大栈大小
    max_stack_size: usize,
    
    /// 最大调用深度
    max_call_depth: usize,
}

impl Default for VMConfig {
    fn default() -> Self {
        VMConfig {
            debug_mode: false,
            max_stack_size: 1024,
            max_call_depth: 256,
        }
    }
}
```

#### 任务 2.2: 指令执行循环

**文件**: `src/vm/executor.rs`

```rust
impl VM {
    /// 执行字节码文件
    pub fn execute(&mut self, bytecode: &BytecodeFile) -> Result<Value, VMError> {
        // 加载常量池和类型表
        self.const_pool = bytecode.const_pool.clone();
        self.type_table = bytecode.type_table.clone();
        
        // 找到入口函数
        let entry_idx = bytecode.header.entry_point as usize;
        let entry_func = &bytecode.code_section.functions[entry_idx];
        
        // 创建入口调用帧
        let mut frame = CallFrame::new(entry_func, 0);
        self.call_stack.push(frame);
        
        // 执行主循环
        self.execute_loop()
    }
    
    /// 指令执行循环
    fn execute_loop(&mut self) -> Result<Value, VMError> {
        loop {
            // 获取当前指令
            let frame = self.call_stack.last_mut().unwrap();
            let ip = frame.ip;
            
            // 解码指令
            let opcode = TypedOpcode::try_from(frame.code[ip])?;
            
            // 执行指令
            frame.ip += 1 + opcode.operand_count() as usize;
            self.execute_instruction(opcode, frame)?;
            
            // 检查是否返回
            if let Some(value) = self.check_return() {
                return value;
            }
        }
    }
    
    /// 执行单条指令
    fn execute_instruction(&mut self, opcode: TypedOpcode, frame: &mut CallFrame) -> Result<(), VMError> {
        match opcode {
            TypedOpcode::Nop => Ok(()),
            TypedOpcode::Mov => self.op_mov(frame),
            TypedOpcode::I64Add => self.op_i64_add(frame),
            // ... 其他指令
        }
    }
}
```

#### 任务 2.3: 调用帧管理

**文件**: `src/vm/frames.rs`

```rust
/// 调用帧
pub struct CallFrame {
    /// 当前执行的函数
    func: FunctionCode,
    
    /// 指令指针
    pub ip: usize,
    
    /// 局部变量槽
    locals: Vec<Value>,
    
    /// 操作数栈
    stack: Vec<Value>,
}

impl CallFrame {
    pub fn new(func: &FunctionCode, entry_ip: usize) -> Self {
        CallFrame {
            func: func.clone(),
            ip: entry_ip,
            locals: vec![Value::Void; func.local_count],
            stack: Vec::with_capacity(64),
        }
    }
}
```

---

### Phase 6.3: 核心指令实现

#### 优先级 1: 必须实现

| 指令 | 实现难度 | 优先级 |
|------|----------|--------|
| `Nop`, `Mov` | ⭐ | P0 |
| `LoadConst`, `LoadLocal`, `StoreLocal`, `LoadArg` | ⭐ | P0 |
| `I64Add`, `I64Sub`, `I64Mul`, `I64Div` | ⭐ | P0 |
| `F64Add`, `F64Sub`, `F64Mul`, `F64Div` | ⭐ | P0 |
| `I64Eq`, `I64Ne`, `I64Lt`, `I64Le`, `I64Gt`, `I64Ge` | ⭐ | P0 |
| `F64Eq`, `F64Ne`, `F64Lt`, `F64Le`, `F64Gt`, `F64Ge` | ⭐ | P0 |
| `Jmp`, `JmpIf`, `JmpIfNot` | ⭐⭐ | P0 |
| `CallStatic`, `ReturnValue`, `Return` | ⭐⭐ | P0 |

#### 优先级 2: 推荐实现

| 指令 | 实现难度 | 优先级 |
|------|----------|--------|
| `GetField`, `SetField` | ⭐⭐ | P1 |
| `NewListWithCap`, `StoreElement`, `LoadElement` | ⭐⭐ | P1 |
| `StringLength`, `StringConcat` | ⭐⭐ | P1 |
| `CallVirt`, `CallDyn` | ⭐⭐⭐ | P1 |

#### 指令执行示例

```rust
// I64Add: dst = src1 + src2
fn op_i64_add(&mut self, frame: &mut CallFrame) -> Result<(), VMError> {
    let src2 = frame.stack.pop().unwrap();
    let src1 = frame.stack.pop().unwrap();
    let dst = src1.as_i64()? + src2.as_i64()?;
    frame.stack.push(Value::Int(dst));
    Ok(())
}

// JmpIfNot: 条件为假时跳转
fn op_jmp_if_not(&mut self, frame: &mut CallFrame, offset: i16) -> Result<(), VMError> {
    let cond = frame.stack.pop().unwrap();
    if !cond.as_bool()? {
        frame.ip = (frame.ip as i32 + offset as i32) as usize;
    }
    Ok(())
}

// CallStatic: 静态函数调用
fn op_call_static(&mut self, frame: &mut CallFrame, func_idx: u32, arg_count: u8) -> Result<(), VMError> {
    // 弹出参数
    let mut args = Vec::new();
    for _ in 0..arg_count {
        args.push(frame.stack.pop().unwrap());
    }
    args.reverse();
    
    // 获取目标函数
    let target_func = &self.get_function(func_idx)?;
    
    // 创建新调用帧
    let mut new_frame = CallFrame::new(target_func, 0);
    
    // 复制参数到局部变量
    for (i, arg) in args.iter().enumerate() {
        if i < target_func.params.len() {
            new_frame.locals[i] = arg.clone();
        }
    }
    
    // 压栈
    self.call_stack.push(new_frame);
    Ok(())
}
```

---

### Phase 6.4: 测试与调优

#### 6.4.1 测试用例

```rust
// src/vm/tests/mod.rs

#[test]
fn test_integer_arithmetic() {
    let bytecode = compile("add(a: Int, b: Int) -> Int = (a, b) => a + b").unwrap();
    let mut vm = VM::new();
    let result = vm.execute(&bytecode).unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_conditional_branch() {
    let code = r#"
        x: Int = 10
        if x > 5 {
            "大于5"
        } else {
            "小于等于5"
        }
    "#;
    let bytecode = compile(code).unwrap();
    let mut vm = VM::new();
    let result = vm.execute(&bytecode).unwrap();
    assert_eq!(result, Value::String("大于5".to_string()));
}

#[test]
fn test_while_loop() {
    let code = r#"
        sum: Int = 0
        i: Int = 0
        while i < 5 {
            sum = sum + i
            i = i + 1
        }
        sum
    "#;
    let bytecode = compile(code).unwrap();
    let mut vm = VM::new();
    let result = vm.execute(&bytecode).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_function_call() {
    let code = r#"
        double(x: Int) -> Int = (x) => x * 2
        double(5)
    "#;
    let bytecode = compile(code).unwrap();
    let mut vm = VM::new();
    let result = vm.execute(&bytecode).unwrap();
    assert_eq!(result, Value::Int(10));
}
```

#### 6.4.2 集成测试

```rust
// tests/integration/vm.rs

#[test]
fn test_hello_world() {
    let code = std::fs::read_to_string("docs/examples/hello.yx").unwrap();
    let bytecode = compile(&code).unwrap();
    let mut vm = VM::new();
    let _ = vm.execute(&bytecode);
    // 验证输出
}
```

---

## 五、文件结构

```
src/vm/
├── mod.rs              # VM 主入口，公开 API
├── executor.rs         # 指令执行循环
├── frames.rs           # CallFrame 实现
├── opcode.rs           # 已完成：TypedOpcode 定义
├── instructions.rs     # 指令编码/解码工具
└── errors.rs           # VMError 定义

src/vm/tests/
├── mod.rs
├── executor.rs
├── frames.rs
└── opcode.rs
```

---

## 六、依赖关系

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           依赖图                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  codegen/mod.rs ─────┐                                                 │
│  codegen/expr.rs ────┼──► bytecode.rs ──► VM ──► tests                 │
│  codegen/stmt.rs ────┘         │                                        │
│                                 │                                        │
│  vm/opcode.rs ─────────────────┘                                        │
│  vm/frames.rs                                                               │
│  vm/executor.rs                                                            │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 七、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 代码生成器缺陷 | VM 无法执行 | 先完善代码生成器 |
| 栈溢出 | 运行时崩溃 | 添加栈大小检查 |
| 无限循环 | 程序挂起 | 添加最大迭代次数 |
| 类型错误 | 运行时 panic | 添加类型检查 |

---

## 八、验收标准

### 8.1 功能验收

| 测试项 | 标准 |
|--------|------|
| 算术运算 | Int/Float 加减乘除正确 |
| 比较运算 | 六种比较操作正确 |
| 条件分支 | if-else 执行正确 |
| 循环 | while 循环正确执行 |
| 函数调用 | 参数传递、返回值正确 |
| 字符串 | 拼接、长度操作正确 |

### 8.2 性能验收

| 测试项 | 标准 |
|--------|------|
| 启动时间 | < 100ms |
| 简单函数调用 | < 1μs |
| 斐波那契 (n=20) | < 100ms |

### 8.3 质量验收

| 测试项 | 标准 |
|--------|------|
| 代码覆盖率 | >= 80% |
| 文档完整性 | 每个公共 API 有文档 |
| 测试通过率 | 100% |

---

## 九、下一步

1. **立即执行**: 完善代码生成器（Phase 6.1）
2. **并行**: 实现 VM 核心框架（Phase 6.2）
3. **按需**: 实现核心指令（Phase 6.3）
4. **最后**: 测试与调优（Phase 6.4）

---

> 「千里之行，始于足下。」  
> MVP 的实现将为后续优化和扩展奠定基础。
