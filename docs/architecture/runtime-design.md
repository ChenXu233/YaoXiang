# YaoXiang 运行时设计文档

> 版本：v1.0.0
> 状态：正式
> 作者：晨煦
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [虚拟机设计](#二虚拟机设计)
3. [并作模型实现](#三并作模型实现)
4. [内存管理](#四内存管理)
5. [调度器设计](#五调度器设计)
6. [标准库集成](#六标准库集成)
7. [性能优化](#七性能优化)

---

## 一、概述

YaoXiang 运行时系统是一个高性能的执行环境，核心特性包括：
- **字节码虚拟机**：执行编译后的指令流
- **并作并发模型**：自动并行化，零认知负担
- **所有权内存管理**：编译时保证安全，运行时零开销
- **多线程调度器**：工作窃取，负载均衡

### 运行时架构总览

```
字节码输入
  ↓
┌─────────────────────────────────────────────────────────┐
│                    虚拟机执行器 (VM)                     │
├─────────────────────────────────────────────────────────┤
│  指令分发 → 执行循环                                    │
│  ↓                                                      │
│  调用帧管理 → 函数调用栈                                │
│  ↓                                                      │
│  值栈管理 → 操作数栈                                    │
│  ↓                                                      │
│  内存访问 → 栈/堆/常量池                                │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                  并作调度器 (Scheduler)                 │
├─────────────────────────────────────────────────────────┤
│  任务生成 → 并作图节点                                   │
│  ↓                                                      │
│  依赖追踪 → DAG 管理                                    │
│  ↓                                                      │
│  工作窃取 → 多线程负载均衡                              │
│  ↓                                                      │
│  自动等待 → 惰性求值                                    │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                  内存管理器 (Memory)                    │
├─────────────────────────────────────────────────────────┤
│  栈分配 → 局部变量                                      │
│  堆分配 → 动态对象                                      │
│  RAII → 自动资源清理                                    │
│  引用计数 → 共享对象                                    │
└─────────────────────────────────────────────────────────┘
  ↓
执行结果
```

---

## 二、虚拟机设计

### 2.1 虚拟机核心结构

```rust
pub struct VirtualMachine {
    // 执行状态
    pub stack: Vec<Value>,              // 操作数栈
    pub frames: Vec<CallFrame>,         // 调用帧栈
    pub ip: usize,                      // 指针指针（当前帧内）
    
    // 内存区域
    pub constant_pool: Vec<Value>,      // 常量池
    pub globals: HashMap<String, Value>, // 全局变量
    
    // 字节码
    pub bytecode: Vec<u8>,              // 指令流
    pub functions: Vec<FunctionHeader>, // 函数表
    
    // 执行上下文
    pub scheduler: Scheduler,           // 并作调度器
    pub heap: Heap,                     // 堆管理器
    
    // 异常处理
    pub exception_handler: Option<ExceptionHandler>,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub func_idx: u32,          // 函数索引
    pub return_addr: u32,       // 返回地址（在字节码中的位置）
    pub base_ptr: u32,          // 栈基址（当前帧在栈中的起始位置）
    pub closure: Option<Closure>, // 闭包环境
}

#[derive(Debug, Clone)]
pub struct FunctionHeader {
    pub name: String,
    pub arity: u8,              // 参数个数
    pub entry_point: u32,       // 入口地址
    pub is_native: bool,        // 是否是原生函数
}
```

### 2.2 执行循环

```rust
impl VirtualMachine {
    pub fn run(&mut self, func_id: FuncId) -> Result<Value, RuntimeError> {
        // 设置初始调用帧
        self.push_call_frame(func_id, 0);
        
        loop {
            // 获取当前指令
            let instr = self.fetch_instruction()?;
            
            match instr {
                // 栈操作
                Instruction::PushConstant(idx) => {
                    let value = self.constant_pool[idx as usize].clone();
                    self.push_stack(value);
                }
                
                Instruction::Pop => {
                    self.pop_stack();
                }
                
                Instruction::Dup => {
                    let top = self.peek_stack(0)?;
                    self.push_stack(top);
                }
                
                // 变量操作
                Instruction::LoadLocal(slot) => {
                    let value = self.get_local(slot)?;
                    self.push_stack(value);
                }
                
                Instruction::StoreLocal(slot) => {
                    let value = self.pop_stack()?;
                    self.set_local(slot, value);
                }
                
                Instruction::LoadGlobal(idx) => {
                    let name = self.get_global_name(idx)?;
                    let value = self.globals.get(&name)
                        .ok_or_else(|| RuntimeError::UndefinedGlobal(name))?;
                    self.push_stack(value.clone());
                }
                
                Instruction::StoreGlobal(idx) => {
                    let value = self.pop_stack()?;
                    let name = self.get_global_name(idx)?;
                    self.globals.insert(name, value);
                }
                
                // 算术运算
                Instruction::Add => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    let result = self.add_values(a, b)?;
                    self.push_stack(result);
                }
                
                Instruction::Sub => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    let result = self.sub_values(a, b)?;
                    self.push_stack(result);
                }
                
                Instruction::Mul => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    let result = self.mul_values(a, b)?;
                    self.push_stack(result);
                }
                
                Instruction::Div => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    let result = self.div_values(a, b)?;
                    self.push_stack(result);
                }
                
                // 比较运算
                Instruction::Eq => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(a == b));
                }
                
                Instruction::Ne => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(a != b));
                }
                
                Instruction::Lt => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(self.compare(a, b)? < 0));
                }
                
                Instruction::Le => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(self.compare(a, b)? <= 0));
                }
                
                Instruction::Gt => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(self.compare(a, b)? > 0));
                }
                
                Instruction::Ge => {
                    let b = self.pop_stack()?;
                    let a = self.pop_stack()?;
                    self.push_stack(Value::Bool(self.compare(a, b)? >= 0));
                }
                
                // 控制流
                Instruction::Jump(addr) => {
                    self.jump(addr);
                }
                
                Instruction::JumpIf(addr) => {
                    let cond = self.pop_stack()?;
                    if self.is_truthy(cond) {
                        self.jump(addr);
                    }
                }
                
                Instruction::Call(func_idx) => {
                    self.call_function(func_idx)?;
                }
                
                Instruction::TailCall(func_idx) => {
                    self.tail_call_function(func_idx)?;
                }
                
                Instruction::Return => {
                    let result = if self.has_return_value()? {
                        Some(self.pop_stack()?)
                    } else {
                        None
                    };
                    
                    if !self.pop_call_frame(result)? {
                        // 主函数返回，结束执行
                        return Ok(result.unwrap_or(Value::Unit));
                    }
                }
                
                // 并发
                Instruction::Spawn(func_idx) => {
                    let task_id = self.spawn_task(func_idx)?;
                    self.push_stack(Value::Task(task_id));
                }
                
                Instruction::Await => {
                    let task_id = self.pop_stack()?;
                    let result = self.await_task(task_id)?;
                    self.push_stack(result);
                }
                
                // 类型操作
                Instruction::Cast => {
                    let target_type = self.pop_type()?;
                    let value = self.pop_stack()?;
                    let result = self.cast_value(value, target_type)?;
                    self.push_stack(result);
                }
                
                Instruction::Is => {
                    let target_type = self.pop_type()?;
                    let value = self.pop_stack()?;
                    let is_match = self.type_check(value, target_type)?;
                    self.push_stack(Value::Bool(is_match));
                }
                
                // 错误处理
                Instruction::Halt => {
                    return Err(RuntimeError::Halted);
                }
                
                _ => {
                    return Err(RuntimeError::UnknownInstruction);
                }
            }
            
            // 检查栈深度限制
            if self.stack.len() > MAX_STACK_DEPTH {
                return Err(RuntimeError::StackOverflow);
            }
        }
    }
    
    fn fetch_instruction(&mut self) -> Result<Instruction, RuntimeError> {
        let frame = self.frames.last()
            .ok_or_else(|| RuntimeError::NoCallFrame)?;
        
        if self.ip >= self.bytecode.len() {
            return Ok(Instruction::Return);
        }
        
        let byte = self.bytecode[self.ip];
        self.ip += 1;
        
        // 指令解码（简化版）
        let instr = match byte {
            0x01 => Instruction::PushConstant(self.read_u32()?),
            0x02 => Instruction::Pop,
            0x03 => Instruction::Dup,
            0x10 => Instruction::LoadLocal(self.read_u8()?),
            0x11 => Instruction::StoreLocal(self.read_u8()?),
            0x12 => Instruction::LoadGlobal(self.read_u32()?),
            0x13 => Instruction::StoreGlobal(self.read_u32()?),
            0x20 => Instruction::Add,
            0x21 => Instruction::Sub,
            0x22 => Instruction::Mul,
            0x23 => Instruction::Div,
            0x30 => Instruction::Eq,
            0x31 => Instruction::Ne,
            0x32 => Instruction::Lt,
            0x33 => Instruction::Le,
            0x34 => Instruction::Gt,
            0x35 => Instruction::Ge,
            0x40 => Instruction::Jump(self.read_u32()?),
            0x41 => Instruction::JumpIf(self.read_u32()?),
            0x42 => Instruction::Call(self.read_u32()?),
            0x43 => Instruction::TailCall(self.read_u32()?),
            0x44 => Instruction::Return,
            0x50 => Instruction::Spawn(self.read_u32()?),
            0x51 => Instruction::Await,
            0x60 => Instruction::Cast,
            0x61 => Instruction::Is,
            0xFF => Instruction::Halt,
            _ => return Err(RuntimeError::UnknownInstruction),
        };
        
        Ok(instr)
    }
    
    fn read_u8(&mut self) -> Result<u8, RuntimeError> {
        if self.ip >= self.bytecode.len() {
            return Err(RuntimeError::UnexpectedEof);
        }
        let byte = self.bytecode[self.ip];
        self.ip += 1;
        Ok(byte)
    }
    
    fn read_u32(&mut self) -> Result<u32, RuntimeError> {
        if self.ip + 3 >= self.bytecode.len() {
            return Err(RuntimeError::UnexpectedEof);
        }
        let bytes = &self.bytecode[self.ip..self.ip + 4];
        let value = u32::from_be_bytes(bytes.try_into().unwrap());
        self.ip += 4;
        Ok(value)
    }
}
```

### 2.3 栈管理

```rust
impl VirtualMachine {
    fn push_stack(&mut self, value: Value) {
        self.stack.push(value);
    }
    
    fn pop_stack(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop()
            .ok_or_else(|| RuntimeError::StackUnderflow)
    }
    
    fn peek_stack(&self, depth: usize) -> Result<Value, RuntimeError> {
        if depth >= self.stack.len() {
            return Err(RuntimeError::StackUnderflow);
        }
        let idx = self.stack.len() - 1 - depth;
        Ok(self.stack[idx].clone())
    }
    
    fn get_local(&self, slot: u8) -> Result<Value, RuntimeError> {
        let frame = self.frames.last()
            .ok_or_else(|| RuntimeError::NoCallFrame)?;
        let idx = (frame.base_ptr + slot as u32) as usize;
        
        if idx >= self.stack.len() {
            return Err(RuntimeError::InvalidLocal);
        }
        
        Ok(self.stack[idx].clone())
    }
    
    fn set_local(&mut self, slot: u8, value: Value) -> Result<(), RuntimeError> {
        let frame = self.frames.last()
            .ok_or_else(|| RuntimeError::NoCallFrame)?;
        let idx = (frame.base_ptr + slot as u32) as usize;
        
        if idx >= self.stack.len() {
            return Err(RuntimeError::InvalidLocal);
        }
        
        self.stack[idx] = value;
        Ok(())
    }
}
```

### 2.4 调用帧管理

```rust
impl VirtualMachine {
    fn push_call_frame(&mut self, func_idx: FuncId, return_addr: u32) {
        let func = &self.functions[func_idx.0 as usize];
        
        let frame = CallFrame {
            func_idx: func_idx.0,
            return_addr,
            base_ptr: self.stack.len() as u32,
            closure: None,
        };
        
        self.frames.push(frame);
        self.ip = func.entry_point as usize;
    }
    
    fn pop_call_frame(&mut self, result: Option<Value>) -> Result<bool, RuntimeError> {
        let frame = self.frames.pop()
            .ok_or_else(|| RuntimeError::NoCallFrame)?;
        
        // 清理参数和局部变量
        let cleanup_count = self.stack.len() - frame.base_ptr as usize;
        self.stack.truncate(frame.base_ptr as usize);
        
        // 如果有返回值，压入栈
        if let Some(ret_val) = result {
            self.push_stack(ret_val);
        }
        
        // 恢复上一帧的执行位置
        if let Some(prev_frame) = self.frames.last() {
            self.ip = prev_frame.return_addr as usize;
            Ok(true)  // 继续执行
        } else {
            Ok(false)  // 主函数返回
        }
    }
    
    fn call_function(&mut self, func_idx: u32) -> Result<(), RuntimeError> {
        let func = &self.functions[func_idx as usize];
        
        if func.is_native {
            // 调用原生函数
            self.call_native(func_idx)?;
        } else {
            // 普通函数调用
            let return_addr = self.ip as u32;
            self.push_call_frame(FuncId(func_idx), return_addr);
        }
        
        Ok(())
    }
    
    fn tail_call_function(&mut self, func_idx: u32) -> Result<(), RuntimeError> {
        // 尾调用优化：复用当前调用帧
        let frame = self.frames.last_mut()
            .ok_or_else(|| RuntimeError::NoCallFrame)?;
        
        // 保存当前栈顶的参数
        let func = &self.functions[func_idx as usize];
        let args: Vec<Value> = self.stack.drain(
            self.stack.len() - func.arity as usize..
        ).collect();
        
        // 清理当前帧的局部变量
        self.stack.truncate(frame.base_ptr as usize);
        
        // 更新帧信息
        frame.func_idx = func_idx;
        self.ip = func.entry_point as usize;
        
        // 重新压入参数
        for arg in args {
            self.push_stack(arg);
        }
        
        Ok(())
    }
    
    fn call_native(&mut self, func_idx: u32) -> Result<(), RuntimeError> {
        // 从栈中获取参数
        let func = &self.functions[func_idx as usize];
        let arg_count = func.arity as usize;
        
        let args = if arg_count > 0 {
            let start = self.stack.len() - arg_count;
            self.stack.drain(start..).collect()
        } else {
            Vec::new()
        };
        
        // 执行原生函数
        let result = self.execute_native(func_idx, args)?;
        
        // 压入结果
        if let Some(value) = result {
            self.push_stack(value);
        }
        
        Ok(())
    }
}
```

---

## 三、并作模型实现

### 3.1 任务与并作图

```rust
// 任务状态
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub func_idx: FuncId,
    pub args: Vec<Value>,
    pub status: TaskStatus,
    pub result: Option<Value>,
    pub deps: Vec<TaskId>,  // 依赖的任务
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,      // 等待依赖
    Ready,        // 可以执行
    Running,      // 执行中
    Completed,    // 已完成
    Failed,       // 失败
}

// 并作图节点
#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: NodeId,
    pub task_id: Option<TaskId>,
    pub deps: Vec<NodeId>,  // 依赖的节点
    pub status: NodeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
}

// 并作图
pub struct Dag {
    nodes: HashMap<NodeId, DagNode>,
    node_counter: u32,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            node_counter: 0,
        }
    }
    
    pub fn create_node(&mut self, task_id: Option<TaskId>, deps: Vec<NodeId>) -> NodeId {
        let id = NodeId(self.node_counter);
        self.node_counter += 1;
        
        let node = DagNode {
            id,
            task_id,
            deps,
            status: if deps.is_empty() {
                NodeStatus::Ready
            } else {
                NodeStatus::Pending
            },
        };
        
        self.nodes.insert(id, node);
        id
    }
    
    pub fn get_ready_nodes(&self) -> Vec<NodeId> {
        self.nodes.values()
            .filter(|n| n.status == NodeStatus::Ready)
            .map(|n| n.id)
            .collect()
    }
    
    pub fn mark_completed(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = NodeStatus::Completed;
            
            // 检查依赖此节点的其他节点
            for other in self.nodes.values_mut() {
                if other.deps.contains(&node_id) {
                    // 移除已完成的依赖
                    other.deps.retain(|&d| d != node_id);
                    // 如果所有依赖都完成，标记为就绪
                    if other.deps.is_empty() {
                        other.status = NodeStatus::Ready;
                    }
                }
            }
        }
    }
    
    pub fn mark_failed(&mut self, node_id: NodeId, error: RuntimeError) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = NodeStatus::Failed;
            // 传播失败到依赖节点
            for other in self.nodes.values_mut() {
                if other.deps.contains(&node_id) {
                    other.status = NodeStatus::Failed;
                }
            }
        }
    }
}
```

### 3.2 并作执行流程

```rust
impl VirtualMachine {
    // spawn 指令的实现
    fn spawn_task(&mut self, func_idx: u32) -> Result<TaskId, RuntimeError> {
        // 从栈中获取参数
        let func = &self.functions[func_idx as usize];
        let arg_count = func.arity as usize;
        
        let args = if arg_count > 0 {
            let start = self.stack.len() - arg_count;
            self.stack.drain(start..).collect()
        } else {
            Vec::new()
        };
        
        // 创建任务
        let task_id = self.scheduler.create_task(FuncId(func_idx), args);
        
        // 创建并作图节点
        let deps = self.detect_dependencies(&args);
        let node_id = self.scheduler.dag.create_node(Some(task_id), deps);
        
        // 如果节点就绪，提交到工作队列
        if self.scheduler.dag.nodes[&node_id].status == NodeStatus::Ready {
            self.scheduler.submit_task(task_id);
        }
        
        Ok(task_id)
    }
    
    // await 指令的实现
    fn await_task(&mut self, task_id: Value) -> Result<Value, RuntimeError> {
        let tid = match task_id {
            Value::Task(id) => id,
            _ => return Err(RuntimeError::TypeMismatch),
        };
        
        // 检查任务状态
        if let Some(task) = self.scheduler.tasks.get(&tid) {
            match task.status {
                TaskStatus::Completed => {
                    return Ok(task.result.clone().unwrap());
                }
                TaskStatus::Failed => {
                    return Err(RuntimeError::TaskFailed);
                }
                TaskStatus::Pending | TaskStatus::Ready | TaskStatus::Running => {
                    // 需要等待
                    // 这里会挂起当前任务，让出执行权
                    self.scheduler.wait_for_task(tid);
                    
                    // 重新调度其他任务
                    self.scheduler.run_to_completion()?;
                    
                    // 再次检查结果
                    if let Some(task) = self.scheduler.tasks.get(&tid) {
                        match task.status {
                            TaskStatus::Completed => {
                                return Ok(task.result.clone().unwrap());
                            }
                            TaskStatus::Failed => {
                                return Err(RuntimeError::TaskFailed);
                            }
                            _ => return Err(RuntimeError::TaskNotReady),
                        }
                    }
                }
            }
        }
        
        Err(RuntimeError::TaskNotFound)
    }
    
    // 检测依赖关系
    fn detect_dependencies(&self, args: &[Value]) -> Vec<NodeId> {
        let mut deps = Vec::new();
        
        for arg in args {
            if let Value::Task(task_id) = arg {
                // 查找任务对应的节点
                if let Some(node_id) = self.scheduler.get_node_for_task(*task_id) {
                    // 检查节点是否已完成
                    if let Some(node) = self.scheduler.dag.nodes.get(&node_id) {
                        if node.status != NodeStatus::Completed {
                            deps.push(node_id);
                        }
                    }
                }
            }
        }
        
        deps
    }
}
```

### 3.3 惰性求值策略

```rust
// 并作值（代理对象）
#[derive(Debug, Clone)]
pub struct AsyncValue {
    pub task_id: TaskId,
    pub value: Option<Value>,
}

impl AsyncValue {
    pub fn get(&self, vm: &VirtualMachine) -> Result<Value, RuntimeError> {
        if let Some(val) = &self.value {
            return Ok(val.clone());
        }
        
        // 触发等待
        let result = vm.await_task(Value::Task(self.task_id))?;
        Ok(result)
    }
}

// 在字节码层面，自动插入等待点
// 例如：访问并作值的字段时
// 原代码：data = fetch_data(); print(data.name)
// 编译后：
//   0. fetch_data() -> Task
//   1. Await -> 实际值
//   2. LoadField "name"
//   3. print
```

---

## 四、内存管理

### 4.1 值类型

```rust
#[derive(Debug, Clone)]
pub enum Value {
    // 原子类型
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    String(String),
    Unit,  // 空值
    
    // 复合类型
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Tuple(Vec<Value>),
    
    // 函数
    Function(FuncId),
    Closure(Closure),
    
    // 并发
    Task(TaskId),
    
    // 引用
    Ref(RefCell<Value>),
    
    // 对象（堆分配）
    Object(HeapPtr),
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub func_idx: FuncId,
    pub env: HashMap<String, Value>,  // 捕获的变量
}

#[derive(Debug, Clone, Copy)]
pub struct RefCell<T>(T);  // 简化的内部可变性
```

### 4.2 堆管理器

```rust
pub struct Heap {
    // 简单的标记清除 GC
    objects: HashMap<HeapPtr, HeapObject>,
    next_ptr: u64,
    allocated_bytes: usize,
    gc_threshold: usize,
}

#[derive(Debug, Clone)]
pub struct HeapObject {
    pub data: ObjectData,
    pub marked: bool,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub enum ObjectData {
    // 大列表
    LargeList(Vec<Value>),
    
    // 大字典
    LargeDict(HashMap<String, Value>),
    
    // 自定义对象
    Custom {
        type_name: String,
        fields: HashMap<String, Value>,
    },
}

impl Heap {
    pub fn allocate(&mut self, data: ObjectData) -> HeapPtr {
        let size = self.estimate_size(&data);
        self.allocated_bytes += size;
        
        let ptr = HeapPtr(self.next_ptr);
        self.next_ptr += 1;
        
        self.objects.insert(ptr, HeapObject {
            data,
            marked: false,
            size,
        });
        
        // 触发 GC
        if self.allocated_bytes > self.gc_threshold {
            self.collect_garbage();
        }
        
        ptr
    }
    
    fn estimate_size(&self, data: &ObjectData) -> usize {
        match data {
            ObjectData::LargeList(vec) => vec.len() * std::mem::size_of::<Value>(),
            ObjectData::LargeDict(map) => map.len() * 64,  // 估算
            ObjectData::Custom { fields, .. } => fields.len() * 64,
        }
    }
    
    pub fn collect_garbage(&mut self) {
        // 标记阶段
        self.mark_roots();
        
        // 清除阶段
        self.sweep();
        
        // 调整阈值
        self.gc_threshold = (self.allocated_bytes as f64 * 1.5) as usize;
    }
    
    fn mark_roots(&mut self) {
        // 标记栈上的对象引用
        // 标记全局变量
        // 标记任务中的值
    }
    
    fn sweep(&mut self) {
        let mut to_remove = Vec::new();
        
        for (ptr, obj) in self.objects.iter_mut() {
            if !obj.marked {
                to_remove.push(*ptr);
                self.allocated_bytes -= obj.size;
            } else {
                obj.marked = false;  // 重置标记
            }
        }
        
        for ptr in to_remove {
            self.objects.remove(&ptr);
        }
    }
}
```

### 4.3 所有权与引用计数

```rust
pub struct Rc<T> {
    ptr: *mut RcBox<T>,
}

struct RcBox<T> {
    count: usize,
    value: T,
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            (*self.ptr).count += 1;
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.ptr).count -= 1;
            if (*self.ptr).count == 0 {
                drop(Box::from_raw(self.ptr));
            }
        }
    }
}

// 用于编译时所有权检查的引用
#[derive(Debug)]
pub struct Ref<T> {
    value: *const T,
    // 记录借用状态
    borrowed: bool,
}

#[derive(Debug)]
pub struct RefMut<T> {
    value: *mut T,
    borrowed: bool,
}
```

---

## 五、调度器设计

### 5.1 工作窃取调度器

```rust
pub struct Scheduler {
    // 工作线程
    workers: Vec<Worker>,
    
    // 全局任务队列
    global_queue: TaskQueue,
    
    // 任务状态
    tasks: HashMap<TaskId, Task>,
    
    // 并作图
    dag: Dag,
    
    // 线程池（用于阻塞操作）
    blocking_pool: ThreadPool,
    
    // 配置
    config: SchedulerConfig,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub num_workers: usize,
    pub work_stealing_threshold: usize,
    pub max_blocking_threads: usize,
}

// 每个线程的工作器
pub struct Worker {
    id: usize,
    local_queue: TaskQueue,
    rng: ThreadRng,  // 用于随机窃取
}

// 任务队列（无锁队列）
pub struct TaskQueue {
    queue: VecDeque<TaskId>,
    // 使用 Arc<Mutex<>> 或更高效的无锁实现
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let mut workers = Vec::with_capacity(config.num_workers);
        for id in 0..config.num_workers {
            workers.push(Worker {
                id,
                local_queue: TaskQueue::new(),
                rng: rand::thread_rng(),
            });
        }
        
        Self {
            workers,
            global_queue: TaskQueue::new(),
            tasks: HashMap::new(),
            dag: Dag::new(),
            blocking_pool: ThreadPool::new(config.max_blocking_threads),
            config,
        }
    }
    
    pub fn create_task(&mut self, func_idx: FuncId, args: Vec<Value>) -> TaskId {
        let task_id = TaskId(self.tasks.len() as u32);
        
        let task = Task {
            id: task_id,
            func_idx,
            args,
            status: TaskStatus::Pending,
            result: None,
            deps: Vec::new(),
        };
        
        self.tasks.insert(task_id, task);
        task_id
    }
    
    pub fn submit_task(&mut self, task_id: TaskId) {
        // 优先提交到全局队列
        self.global_queue.push(task_id);
        
        // 唤醒工作线程
        self.notify_workers();
    }
    
    pub fn run_to_completion(&mut self) -> Result<(), RuntimeError> {
        // 检查是否有就绪节点
        let ready_nodes = self.dag.get_ready_nodes();
        
        for node_id in ready_nodes {
            if let Some(node) = self.dag.nodes.get(&node_id) {
                if let Some(task_id) = node.task_id {
                    self.submit_task(task_id);
                }
            }
        }
        
        // 工作线程执行任务
        while !self.is_idle() {
            for worker in &mut self.workers {
                if let Some(task_id) = self.steal_or_get() {
                    self.execute_task(worker, task_id)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn execute_task(&mut self, worker: &Worker, task_id: TaskId) -> Result<(), RuntimeError> {
        // 获取任务
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound)?;
        
        if task.status != TaskStatus::Ready {
            return Ok(());
        }
        
        task.status = TaskStatus::Running;
        
        // 在虚拟机中执行
        let mut vm = VirtualMachine::new(self);
        let result = vm.run(task.func_idx);
        
        // 更新任务状态
        match result {
            Ok(value) => {
                task.result = Some(value);
                task.status = TaskStatus::Completed;
            }
            Err(e) => {
                task.status = TaskStatus::Failed;
                return Err(e);
            }
        }
        
        // 标记并作图节点完成
        if let Some(node_id) = self.get_node_for_task(task_id) {
            self.dag.mark_completed(node_id);
            
            // 检查是否有新就绪的任务
            let ready_nodes = self.dag.get_ready_nodes();
            for node_id in ready_nodes {
                if let Some(node) = self.dag.nodes.get(&node_id) {
                    if let Some(tid) = node.task_id {
                        self.submit_task(tid);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn steal_or_get(&mut self) -> Option<TaskId> {
        // 1. 尝试从本地队列获取
        if let Some(task) = self.global_queue.pop() {
            return Some(task);
        }
        
        // 2. 尝试从其他工作线程窃取
        for worker in &mut self.workers {
            if let Some(task) = worker.local_queue.steal() {
                return Some(task);
            }
        }
        
        None
    }
    
    fn is_idle(&self) -> bool {
        self.global_queue.is_empty() &&
        self.workers.iter().all(|w| w.local_queue.is_empty())
    }
    
    pub fn wait_for_task(&mut self, task_id: TaskId) {
        // 将当前任务挂起，加入等待队列
        // 当 task_id 完成时，恢复执行
    }
    
    pub fn get_node_for_task(&self, task_id: TaskId) -> Option<NodeId> {
        for (node_id, node) in self.dag.nodes.iter() {
            if let Some(tid) = node.task_id {
                if tid == task_id {
                    return Some(*node_id);
                }
            }
        }
        None
    }
}
```

### 5.2 阻塞操作处理

```rust
impl Scheduler {
    // @blocking 注解的实现
    pub fn execute_blocking<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // 使用专用线程池执行阻塞操作
        self.blocking_pool.execute(f)
    }
    
    // 文件 I/O 示例
    pub fn read_file(&mut self, path: String) -> Result<Value, RuntimeError> {
        // 在阻塞线程池中执行
        let result = self.execute_blocking(move || {
            std::fs::read_to_string(path)
        });
        
        match result {
            Ok(content) => Ok(Value::String(content)),
            Err(e) => Err(RuntimeError::IoError(e)),
        }
    }
}
```

---

## 六、标准库集成

### 6.1 原生函数注册

```rust
pub struct NativeRegistry {
    natives: HashMap<String, NativeFunction>,
}

type NativeFunction = fn(&[Value]) -> Result<Value, RuntimeError>;

impl NativeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            natives: HashMap::new(),
        };
        
        // 注册标准库函数
        registry.register("print", print);
        registry.register("println", println);
        registry.register("panic", panic);
        
        // 数学函数
        registry.register("sqrt", sqrt);
        registry.register("abs", abs);
        registry.register("pow", pow);
        
        // 字符串操作
        registry.register("len", len);
        registry.register("concat", concat);
        registry.register("substring", substring);
        
        // 类型转换
        registry.register("to_int", to_int);
        registry.register("to_string", to_string);
        
        registry
    }
    
    pub fn register(&mut self, name: &str, func: NativeFunction) {
        self.natives.insert(name.to_string(), func);
    }
    
    pub fn get(&self, name: &str) -> Option<NativeFunction> {
        self.natives.get(name).copied()
    }
}

// 原生函数实现示例
fn print(args: &[Value]) -> Result<Value, RuntimeError> {
    for arg in args {
        print!("{}", arg);
    }
    Ok(Value::Unit)
}

fn println(args: &[Value]) -> Result<Value, RuntimeError> {
    for arg in args {
        print!("{}", arg);
    }
    println!();
    Ok(Value::Unit)
}

fn sqrt(args: &[Value]) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.sqrt())),
        Some(Value::Int(i)) => Ok(Value::Float((i as f64).sqrt())),
        _ => Err(RuntimeError::TypeMismatch),
    }
}

fn concat(args: &[Value]) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for arg in args {
        match arg {
            Value::String(s) => result.push_str(s),
            _ => return Err(RuntimeError::TypeMismatch),
        }
    }
    Ok(Value::String(result))
}
```

### 6.2 并发原语

```rust
// Channel 实现
#[derive(Debug, Clone)]
pub struct Channel {
    sender: Sender,
    receiver: Receiver,
}

#[derive(Debug, Clone)]
pub struct Sender {
    channel_id: ChannelId,
}

#[derive(Debug, Clone)]
pub struct Receiver {
    channel_id: ChannelId,
}

impl Channel {
    pub fn new() -> Self {
        // 创建无界或有界队列
        Self {
            sender: Sender { channel_id: ChannelId(0) },
            receiver: Receiver { channel_id: ChannelId(0) },
        }
    }
    
    pub fn send(&self, value: Value) -> Result<(), RuntimeError> {
        // 将值放入队列
        // 如果接收者在等待，唤醒它
        Ok(())
    }
    
    pub fn recv(&self) -> Result<Value, RuntimeError> {
        // 从队列获取值
        // 如果队列为空，挂起当前任务
        Ok(Value::Unit)
    }
}

// Mutex 实现
#[derive(Debug, Clone)]
pub struct Mutex {
    locked: AtomicBool,
    owner: Option<TaskId>,
    waiters: Vec<TaskId>,
}

impl Mutex {
    pub fn lock(&self) -> Result<MutexGuard, RuntimeError> {
        // 如果已锁定，挂起当前任务
        // 否则获取锁
        Ok(MutexGuard { mutex: self.clone() })
    }
}

#[derive(Debug)]
pub struct MutexGuard {
    mutex: Mutex,
}

impl Drop for MutexGuard {
    fn drop(&mut self) {
        // 释放锁，唤醒等待者
    }
}
```

---

## 七、性能优化

### 7.1 虚拟机优化

#### 7.1.1 指令分发优化

```rust
// 使用 computed goto（GCC/Clang 扩展）
#[cfg(any(target_os = "linux", target_os = "macos"))]
impl VirtualMachine {
    pub fn run_fast(&mut self) {
        static mut DISPATCH_TABLE: [&&'static [u8]; 256] = [
            &&OP_PUSH_CONST,
            &&OP_POP,
            &&OP_DUP,
            // ...
        ];
        
        unsafe {
            goto *DISPATCH_TABLE[self.bytecode[self.ip] as usize];
        }
        
        OP_PUSH_CONST: {
            // ...
            goto *DISPATCH_TABLE[self.bytecode[self.ip] as usize];
        }
        
        OP_POP: {
            // ...
            goto *DISPATCH_TABLE[self.bytecode[self.ip] as usize];
        }
    }
}

// Windows 使用 switch 优化
#[cfg(target_os = "windows")]
impl VirtualMachine {
    pub fn run(&mut self) {
        loop {
            match self.bytecode[self.ip] {
                OP_PUSH_CONST => {
                    // ...
                }
                OP_POP => {
                    // ...
                }
                // ...
            }
        }
    }
}
```

#### 7.1.2 内联缓存优化

```rust
pub struct MethodCache {
    cache: HashMap<(TypeId, String), FuncId>,
}

impl MethodCache {
    pub fn lookup(&mut self, type_id: TypeId, method: &str) -> Option<FuncId> {
        let key = (type_id, method.to_string());
        
        if let Some(cached) = self.cache.get(&key) {
            return Some(*cached);
        }
        
        // 缓存未命中，查找并缓存
        let func_id = self.find_method(type_id, method)?;
        self.cache.insert(key, func_id);
        Some(func_id)
    }
    
    fn find_method(&self, type_id: TypeId, method: &str) -> Option<FuncId> {
        // 在类型的方法表中查找
        // 支持继承和接口
        None
    }
}
```

### 7.2 并发优化

#### 7.2.1 无锁队列

```rust
use crossbeam::queue::SegQueue;

pub struct LockFreeTaskQueue {
    queue: SegQueue<TaskId>,
}

impl LockFreeTaskQueue {
    pub fn push(&self, task: TaskId) {
        self.queue.push(task);
    }
    
    pub fn pop(&self) -> Option<TaskId> {
        self.queue.pop()
    }
    
    pub fn steal(&self) -> Option<TaskId> {
        // 工作窃取算法
        self.queue.pop()
    }
}
```

#### 7.2.2 任务窃取优化

```rust
impl Worker {
    pub fn steal(&mut self) -> Option<TaskId> {
        // 随机选择其他工作线程
        if self.workers.is_empty() {
            return None;
        }
        
        let target = self.rng.gen_range(0..self.workers.len());
        let target_worker = &mut self.workers[target];
        
        // 尝试窃取一半任务
        let steal_count = (target_worker.local_queue.len() + 1) / 2;
        
        let mut stolen = Vec::new();
        for _ in 0..steal_count {
            if let Some(task) = target_worker.local_queue.pop() {
                stolen.push(task);
            } else {
                break;
            }
        }
        
        // 将窃取的任务加入本地队列
        for task in stolen {
            self.local_queue.push(task);
        }
        
        self.local_queue.pop()
    }
}
```

### 7.3 JIT 编译（未来扩展）

```rust
pub struct JITCompiler {
    // 热点检测
    hotness_threshold: u32,
    call_counts: HashMap<FuncId, u32>,
    
    // 编译缓存
    compiled: HashMap<FuncId, *const u8>,
    
    // 代码生成器
    codegen: CraneliftCodegen,  // 或 LLVM
}

impl JITCompiler {
    pub fn record_call(&mut self, func_id: FuncId) {
        let count = self.call_counts.entry(func_id).or_insert(0);
        *count += 1;
        
        if *count > self.hotness_threshold {
            self.compile(func_id);
        }
    }
    
    pub fn compile(&mut self, func_id: FuncId) {
        // 1. 获取 IR
        // 2. 生成机器码
        // 3. 分配可执行内存
        // 4. 缓存函数指针
    }
    
    pub fn get_compiled(&self, func_id: FuncId) -> Option<*const u8> {
        self.compiled.get(&func_id).copied()
    }
}
```

---

## 八、性能基准

### 8.1 微基准测试

```rust
// 计算性能指标
pub struct PerformanceMetrics {
    // 虚拟机
    pub instructions_per_second: f64,
    pub stack_operation_latency: f64,  // ns
    pub function_call_overhead: f64,   // ns
    
    // 并发
    pub task_spawn_latency: f64,       // ns
    pub context_switch_time: f64,      // ns
    pub work_stealing_efficiency: f64, // %
    
    // 内存
    pub allocation_throughput: f64,    // MB/s
    pub gc_pause_time: f64,            // ms
}

// 基准测试示例
#[cfg(test)]
mod benchmarks {
    use super::*;
    use test::Bencher;
    
    #[bench]
    fn bench_task_spawn(b: &mut Bencher) {
        let mut vm = VirtualMachine::new();
        b.iter(|| {
            vm.spawn_task(FuncId(0)).unwrap();
        });
    }
    
    #[bench]
    fn bench_work_stealing(b: &mut Bencher) {
        let scheduler = Scheduler::new(SchedulerConfig {
            num_workers: 4,
            ..Default::default()
        });
        
        b.iter(|| {
            // 提交多个任务
            for _ in 0..100 {
                scheduler.submit_task(TaskId(0));
            }
            scheduler.run_to_completion().unwrap();
        });
    }
}
```

---

## 九、总结

YaoXiang 运行时系统采用现代并发和内存管理技术：

### 核心优势

1. **高性能虚拟机**
   - 优化的指令分发
   - 紧凑的字节码格式
   - 内联缓存加速

2. **并作并发模型**
   - 零认知负担的自动并行
   - 工作窃取负载均衡
   - 依赖图自动管理

3. **高效内存管理**
   - 编译时所有权保证
   - 最小化运行时开销
   - 可选的 GC 隔离

4. **可扩展架构**
   - 模块化设计
   - 易于添加新指令
   - 支持未来 JIT

### 关键性能指标（目标）

- **虚拟机执行**：10-50 亿指令/秒
- **任务创建**：< 100ns
- **上下文切换**：< 1μs
- **GC 暂停**：< 1ms（可选）
- **内存开销**：每个任务 < 1KB

**最后更新**：2025-01-04
