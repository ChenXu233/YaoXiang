# Task 8.1: Runtime 值类型系统

> **优先级**: P0
> **状态**: ⬜ 待开始
> **模块**: `src/core/value.rs`
> **依赖**: phase-05-ownership（所有权模型结果）

## 功能描述

实现 `RuntimeValue` 类型，统一表示 YaoXiang 程序在运行时持有的所有值。

### 核心职责

1. **值表示**：统一表示所有 YaoXiang 类型（Int, Float, Bool, String, Struct, Enum, Array, ref T 等）
2. **类型操作**：值到类型的映射、类型断言、类型查询
3. **内存布局**：定义值在内存中的表示方式
4. **所有权语义**：支持 Move（零拷贝）、ref（Arc）、clone（复制）
5. **与所有权模型集成**：`phase-05-ownership` 检查通过后，Runtime 只需正确表示值

### 设计原则

> **核心洞察**：`RuntimeValue` 是**数据表示层**，不负责所有权检查。
> - **所有权检查**：在 `phase-05-ownership` 完成，编译期静态分析
> - **RuntimeValue**：只需正确表示所有权语义（Move/ref/clone）的运行时效果

## 值类型层次

```rust
/// 值类型枚举（用于类型查询和断言）
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    /// 空值
    Unit,
    /// 布尔值
    Bool,
    /// 整数类型
    Int(IntWidth),
    /// 浮点数类型
    Float(FloatWidth),
    /// 字符（Unicode 码点）
    Char,
    /// 字符串（引用，Arc<str>）
    String,
    /// 字节数组
    Bytes,
    /// 元组
    Tuple(Vec<ValueType>),
    /// 数组（定长）
    Array {
        element: Box<ValueType>,
    },
    /// 动态数组
    List,
    /// 字典/映射
    Dict,
    /// 用户定义类型（结构体）
    Struct(TypeId),
    /// 联合类型（枚举）
    Enum(TypeId),
    /// 函数类型
    Function(FunctionId),
    /// 引用类型（ref T）
    Ref(Box<ValueType>),
    /// Arc 引用（线程安全，ref T 的运行时实现）
    Arc(Box<ValueType>),
    /// 异步值（用于并作模型）
    Async(Box<ValueType>),
    /// 裸指针（*T，只在 unsafe 块中使用）
    Ptr(PtrKind),
}

/// 整数宽度
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntWidth {
    I8, I16, I32, I64, I128, ISize,
    U8, U16, U32, U64, U128, USize,
}

/// 浮点宽度
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatWidth {
    F32, F64,
}

/// 指针类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PtrKind {
    Const,  // *const T
    Mut,    // *mut T
}
```

## Handle 系统（堆分配引用）

```rust
/// 句柄类型（引用堆分配存储）
///
/// Tuple/Array/List/Dict/Struct 使用 Handle 引用堆数据：
/// - 零拷贝 Move（只移动指针）
/// - 高效 Clone（共享底层数据或浅拷贝）
/// - 避免内存碎片
///
/// # 设计说明
/// Handle 是对堆分配数据的轻量级引用，类似于指针但更安全：
/// - 不直接暴露原始指针
/// - 自动跟踪分配状态
/// - 支持零拷贝语义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub(crate) usize);
```

## RuntimeValue 结构

```rust
/// Runtime 值（核心数据结构）
///
/// # 设计说明
/// - 使用 `enum` 而非 struct，便于模式匹配
/// - Arc/Ref 用于共享所有权，值本身不包含所有权状态
/// - 所有权语义由编译期保证，RuntimeValue 只需正确表示
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    /// 空值
    Unit,

    /// 布尔值（小对象，直接存储）
    Bool(bool),

    /// 整数（小对象，直接存储）
    Int(i64),

    /// 浮点数（小对象，直接存储）
    Float(f64),

    /// 字符（Unicode 码点）
    Char(u32),

    /// 字符串（共享字符串，使用 Arc<str> 实现 ref 语义）
    String(Arc<str>),

    /// 字节数组
    Bytes(Arc<[u8]>),

    /// 元组（通过 Handle 引用堆分配数据）
    Tuple(Handle),

    /// 定长数组（通过 Handle 引用堆分配数据）
    Array(Handle),

    /// 动态数组（通过 Handle 引用堆分配数据）
    List(Handle),

    /// 字典（通过 Handle 引用堆分配数据）
    Dict(Handle),

    /// 结构体实例
    Struct {
        /// 类型 ID（用于类型查询和字段访问）
        type_id: TypeId,
        /// 字段值（通过 Handle 引用堆分配数据）
        fields: Handle,
        /// 虚函数表（方法名 -> 函数）
        vtable: Vec<(String, FunctionValue)>,
    },

    /// 枚举变体
    Enum {
        /// 类型 ID
        type_id: TypeId,
        /// 变体索引
        variant_id: u32,
        /// 变体载荷（无载荷时为 Unit）
        payload: Box<RuntimeValue>,
    },

    /// 函数闭包（捕获环境）
    Function(FunctionValue),

    /// 线程安全引用计数（ref T 关键字的运行时实现）
    ///
    /// # 设计说明
    /// `ref T` 关键字在运行时使用 Arc 实现：
    /// - 编译期检查：跨 spawn 传递 ref 会检测循环引用
    /// - 运行时：使用 Arc 自动管理引用计数
    /// - RFC-009: `ref p` 等价于 `Arc::new(p)`
    /// - 使用 `Arc<Arc<RuntimeValue>>` 打破递归类型
    Arc(Arc<RuntimeValue>),

    /// 异步值（用于并作模型）
    ///
    /// # 懒求值
    /// Async 值在创建时不立即计算，而是在首次使用时触发计算
    Async(AsyncValue),

    /// 裸指针（只用于 unsafe 块）
    ///
    /// # 安全性
    /// - 只能在 unsafe 块中使用
    /// - 用户保证不悬空、不释放后使用
    Ptr {
        kind: PtrKind,
        address: usize,
        type_id: TypeId,
    },
}

/// 函数值（闭包）
#[derive(Debug, Clone)]
pub struct FunctionValue {
    /// 函数 ID
    pub func_id: FunctionId,
    /// 捕获的环境变量（闭包）
    pub env: Vec<RuntimeValue>,
}

/// 异步值
#[derive(Debug, Clone)]
pub struct AsyncValue {
    /// 实际值或计算任务
    pub state: AsyncState,
    /// 值类型（用于类型检查）
    pub value_type: ValueType,
}

/// 异步状态
#[derive(Debug, Clone)]
pub enum AsyncState {
    /// 同步就绪的值
    Ready(RuntimeValue),
    /// 待计算的任务（懒求值）
    Pending(TaskId),
    /// 计算出错
    Error(RuntimeValue),
}

/// 类型 ID（用于运行时类型查询）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// 函数 ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

/// 任务 ID（用于异步调度）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub u32);
```

## 内存布局

```rust
impl RuntimeValue {
    /// 获取值的内存布局（用于分配器）
    pub fn layout(&self) -> Layout {
        match self {
            RuntimeValue::Unit => Layout::new::<()>(),
            RuntimeValue::Bool(_) => Layout::new::<bool>(),
            RuntimeValue::Int(_) => Layout::new::<i64>(),
            RuntimeValue::Float(_) => Layout::new::<f64>(),
            RuntimeValue::Char(_) => Layout::new::<u32>(),
            // 字符串和字节数组：Arc 布局
            RuntimeValue::String(_) => Layout::new::<Arc<str>>(),
            RuntimeValue::Bytes(_) => Layout::new::<Arc<[u8]>>(),
            // Handle 类型：Handle 布局（4字节）
            RuntimeValue::Tuple(_) | RuntimeValue::Array(_) | RuntimeValue::List(_) => {
                Layout::new::<Handle>()
            }
            // 字典：Handle 布局
            RuntimeValue::Dict(_) => Layout::new::<Handle>(),
            // 结构体：Handle 布局
            RuntimeValue::Struct { .. } => Layout::new::<Handle>(),
            // 枚举：变体索引 + 载荷
            RuntimeValue::Enum { .. } => Layout::new::<(u32, Box<RuntimeValue>)>(),
            // Arc：Arc 布局
            RuntimeValue::Arc(_) => Layout::new::<Arc<RuntimeValue>>(),
            // 异步值：AsyncState 布局
            RuntimeValue::Async(_) => Layout::new::<AsyncState>(),
            // 裸指针
            RuntimeValue::Ptr { .. } => Layout::new::<usize>(),
            // 函数值
            RuntimeValue::Function(_) => Layout::new::<FunctionValue>(),
        }
    }
}
```

## 类型查询方法

```rust
impl RuntimeValue {
    /// 获取值的静态类型
    ///
    /// # Arguments
    /// * `heap` - 可选的堆引用，用于处理集合类型的完整类型信息
    ///
    /// 当提供 heap 时，集合类型（Tuple、Array）会解析其元素类型。
    /// 当为 None 时，集合类型返回简化类型。
    pub fn value_type(&self, heap: Option<&Heap>) -> ValueType {
        match self {
            RuntimeValue::Unit => ValueType::Unit,
            RuntimeValue::Bool(_) => ValueType::Bool,
            RuntimeValue::Int(_) => ValueType::Int(IntWidth::I64),
            RuntimeValue::Float(_) => ValueType::Float(FloatWidth::F64),
            RuntimeValue::Char(_) => ValueType::Char,
            RuntimeValue::String(_) => ValueType::String,
            RuntimeValue::Bytes(_) => ValueType::Bytes,
            // Handle 类型：优先使用 heap 获取完整类型
            RuntimeValue::Tuple(handle) => {
                if let Some(h) = heap {
                    if let Some(HeapValue::Tuple(items)) = h.get(*handle) {
                        return ValueType::Tuple(items.iter().map(|v| v.value_type(heap)).collect());
                    }
                }
                ValueType::Tuple(vec![])
            }
            RuntimeValue::Array(handle) => {
                if let Some(h) = heap {
                    if let Some(HeapValue::Array(items)) = h.get(*handle) {
                        return ValueType::Array {
                            element: Box::new(items.first().map(|v| v.value_type(heap)).unwrap_or(ValueType::Unit)),
                        };
                    }
                }
                ValueType::Array { element: Box::new(ValueType::Unit) }
            }
            RuntimeValue::List(_) => ValueType::List,
            RuntimeValue::Dict(_) => ValueType::Dict,
            RuntimeValue::Struct { type_id, .. } => ValueType::Struct(*type_id),
            RuntimeValue::Enum { type_id, .. } => ValueType::Enum(*type_id),
            RuntimeValue::Function(f) => ValueType::Function(f.func_id),
            RuntimeValue::Arc(inner) => ValueType::Arc(Box::new(inner.value_type(heap))),
            RuntimeValue::Async(v) => ValueType::Async(Box::new(v.value_type.clone())),
            RuntimeValue::Ptr { kind, .. } => ValueType::Ptr(*kind),
        }
    }

    /// 获取值的静态类型（简便方法，不使用 heap）
    ///
    /// 对于集合类型（Tuple、Array），返回简化类型。
    /// 使用 `value_type(Some(heap))` 获取完整类型信息。
    pub fn value_type_simple(&self) -> ValueType {
        self.value_type(None)
    }

    /// 检查值类型
    pub fn is_type(&self, ty: &ValueType) -> bool {
        &self.value_type(None) == ty
    }

    /// 获取枚举变体 ID
    pub fn enum_variant_id(&self) -> Option<u32> {
        match self {
            RuntimeValue::Enum { variant_id, .. } => Some(*variant_id),
            _ => None,
        }
    }

    /// 获取枚举载荷
    pub fn enum_payload(&self) -> Option<&RuntimeValue> {
        match self {
            RuntimeValue::Enum { payload, .. } => Some(payload),
            _ => None,
        }
    }

    /// 获取结构体字段
    pub fn struct_field(&self, index: usize) -> Option<&RuntimeValue> {
        match self {
            RuntimeValue::Struct { fields, .. } => fields.get(index),
            _ => None,
        }
    }

    /// 获取 Arc 内部值
    pub fn as_arc(&self) -> Option<&RuntimeValue> {
        match self {
            Arc(inner) => Some(inner),
            _ => None,
        }
    }

    /// 转换为布尔
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            RuntimeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn to_int(&self) -> Option<i64> {
        match self {
            RuntimeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 转换为浮点数
    pub fn to_float(&self) -> Option<f64> {
        match self {
            RuntimeValue::Float(f) => Some(*f),
            _ => None,
        }
    }
}
```

## 与所有权模型集成

```rust
/// RuntimeValue 的所有权操作
impl RuntimeValue {
    /// Move：转移所有权（零拷贝，只是指针移动）
    ///
    /// # 说明
    /// - 赋值、传参、返回时自动发生
    /// - 原值失效，不能再使用
    pub fn move_into(self) -> Self {
        self // 值直接转移，零拷贝
    }

    /// Clone：显式复制（用户调用 clone()）
    ///
    /// # 说明
    /// - 深拷贝整个值
    /// - 性能开销取决于值的大小
    pub fn clone(&self) -> Self {
        match self {
            RuntimeValue::Unit => RuntimeValue::Unit,
            RuntimeValue::Bool(b) => RuntimeValue::Bool(*b),
            RuntimeValue::Int(i) => RuntimeValue::Int(*i),
            RuntimeValue::Float(f) => RuntimeValue::Float(*f),
            RuntimeValue::Char(c) => RuntimeValue::Char(*c),
            // Arc 类型共享底层数据
            RuntimeValue::String(s) => RuntimeValue::String(s.clone()),
            RuntimeValue::Bytes(b) => RuntimeValue::Bytes(b.clone()),
            // Vec 需要深拷贝
            RuntimeValue::Tuple(v) => RuntimeValue::Tuple(v.iter().map(|x| x.clone()).collect()),
            RuntimeValue::Array(v) => RuntimeValue::Array(v.iter().map(|x| x.clone()).collect()),
            RuntimeValue::List(v) => RuntimeValue::List(v.iter().map(|x| x.clone()).collect()),
            RuntimeValue::Dict(m) => RuntimeValue::Dict(m.clone()),
            RuntimeValue::Struct { type_id, fields } => RuntimeValue::Struct {
                type_id: *type_id,
                fields: fields.iter().map(|x| x.clone()).collect(),
            },
            RuntimeValue::Enum { type_id, variant_id, payload } => RuntimeValue::Enum {
                type_id: *type_id,
                variant_id: *variant_id,
                payload: Box::new((**payload).clone()),
            },
            RuntimeValue::Function(f) => RuntimeValue::Function(f.clone()),
            // Arc 是 RuntimeValue 的包装，克隆 Arc 本身
            RuntimeValue::Arc(arc) => RuntimeValue::Arc(arc.clone()),
            RuntimeValue::Async(a) => RuntimeValue::Async(a.clone()),
            RuntimeValue::Ptr { kind, address, type_id } => RuntimeValue::Ptr {
                kind: *kind,
                address: *address,
                type_id: *type_id,
            },
        }
    }

    /// 转换为 Arc（ref 关键字的运行时实现）
    pub fn into_arc(self) -> Self {
        Arc(self)
    }

    /// 从 RuntimeValue 获取 Arc 内部值
    pub fn as_arc(&self) -> Option<&RuntimeValue> {
        match self {
            Arc(v) => Some(v),
            _ => None,
        }
    }

    /// 检查是否是 Arc（ref 关键字）
    pub fn is_arc(&self) -> bool {
        matches!(self, Arc(_))
    }
}
```

## 与 RFC-009 对照

| RFC-009 设计 | RuntimeValue 表示 | 说明 |
|-------------|-------------------|------|
| Move 语义 | `RuntimeValue` 赋值 | 零拷贝，指针移动 |
| `ref` 关键字 | `RuntimeValue::Arc` | `Arc(RuntimeValue)`，RFC-009 规定 `ref p` = `Arc::new(p)` |
| `ref T` 类型 | `ValueType::Arc(T)` | 类型系统表示 |
| `clone()` | `RuntimeValue::clone()` | 深拷贝方法 |
| `*T` 裸指针 | `RuntimeValue::Ptr` | unsafe 块中使用 |
| Send/Sync | 自动满足 | Arc 线程安全 |
| 跨任务循环检测 | 编译期完成 | phase-05-ownership |

## 模块结构

```
src/middle/value/
├── mod.rs                # 模块入口
├── runtime_value.rs      # RuntimeValue 与 ValueType 定义
├── heap.rs               # 堆分配系统（Handle 实现）
└── tests/
    ├── mod.rs
    ├── primitives.rs     # 基础类型测试
    ├── struct_enum.rs    # 结构体和枚举测试
    ├── ref_arc.rs        # ref/Arc 测试
    ├── clone.rs          # clone() 测试
    └── handle.rs         # Handle 类型测试
```

> **设计说明**：
> - `RuntimeValue::Arc` 使用 `Arc<Arc<RuntimeValue>>` 实现（RFC-009 规定 `ref p` = `Arc::new(p)`）
> - `RuntimeValue::Ref` 已删除，单线程场景也使用 `Arc`
> - Tuple/Array/List/Dict/Struct 使用 `Handle` 引用堆分配数据
> - 集合类型使用 Handle 而非 Vec，实现零拷贝 Move 和高效 Clone

## 验收测试

```rust
// test_value_type.yx 等价测试

#[test]
fn test_primitive_values() {
    // Int
    let v = RuntimeValue::Int(42);
    assert_eq!(v.value_type(), ValueType::Int(IntWidth::I64));
    assert_eq!(v.to_int(), Some(42));

    // Float
    let v = RuntimeValue::Float(3.14);
    assert_eq!(v.value_type(), ValueType::Float(FloatWidth::F64));

    // Bool
    assert!(RuntimeValue::Bool(true).to_bool().unwrap());
}

#[test]
fn test_struct_value() {
    // type Point = Point(x: Float, y: Float)
    let p = RuntimeValue::Struct {
        type_id: point_type_id,
        fields: vec![RuntimeValue::Float(1.0), RuntimeValue::Float(2.0)],
    };

    assert_eq!(p.struct_field(0), Some(&RuntimeValue::Float(1.0)));
}

#[test]
fn test_enum_value() {
    // type Result[T, E] = ok(T) | err(E)
    let ok = RuntimeValue::Enum {
        type_id: result_type_id,
        variant_id: 0,  // ok
        payload: Box::new(RuntimeValue::Int(42)),
    };

    assert_eq!(ok.enum_variant_id(), Some(0));
}

#[test]
fn test_ref_arc() {
    // ref 关键字 → Arc(RuntimeValue)
    let inner = RuntimeValue::Int(42);
    let arc = RuntimeValue::Arc(inner);

    // Arc 可以克隆（引用计数增加）
    let arc2 = arc.clone();
    assert!(matches!(arc, RuntimeValue::Arc(_)));
    assert!(matches!(arc2, RuntimeValue::Arc(_)));

    // 从 Arc 获取内部值
    assert_eq!(arc.as_arc().unwrap().to_int(), Some(42));
}

#[test]
fn test_clone() {
    let v1 = RuntimeValue::Int(42);
    let v2 = v1.clone();

    assert_eq!(v1.to_int(), Some(42));
    assert_eq!(v2.to_int(), Some(42));
}

#[test]
fn test_async_value() {
    // 同步就绪的 Async 值
    let async_val = RuntimeValue::Async(AsyncValue {
        state: AsyncState::Ready(RuntimeValue::Int(42)),
        value_type: ValueType::Int(IntWidth::I64),
    });

    // 待计算的 Async 值
    let pending = RuntimeValue::Async(AsyncValue {
        state: AsyncState::Pending(TaskId(1)),
        value_type: ValueType::Int(IntWidth::I64),
    });
}
```

> **设计说明**：验收测试中的 `point_type_id`、`result_type_id`、`TaskId` 需要在实际实现时定义。

## 依赖关系

```
phase-05-ownership ──► task-08-01-value-type
       │                     │
       └── 所有权检查 ───────► │
                             └── Runtime 表示 ──► phase-09-dag
```

## 修改记录

| 日期 | 修改内容 | 原因 |
|------|----------|------|
| 2026-01-20 | 删除 `RuntimeValue::Ref`，统一使用 `Arc` | RFC-009 规定 `ref` 关键字等价于 `Arc` |
| 2026-01-20 | `Arc(Arc<RuntimeValue>)` 改为 `Arc(RuntimeValue)` | 消除双重 Arc 冗余 |
| 2026-01-20 | 移除 `ValueType::Array.length` 字段 | 类型只描述结构，不描述实例大小 |
| 2026-01-20 | 移除 `RuntimeValue::Array` 的命名结构 | 与其他集合类型保持一致使用 `Vec` |
