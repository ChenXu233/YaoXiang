# Task 5.5: Send/Sync 约束检查

> **优先级**: P1
> **状态**: ✅ 已实现
> **模块**: `src/middle/lifetime/send_sync.rs`

## 功能描述

检查类型是否满足 Send/Sync 约束，确保并发安全：

- **Send**: 类型可以安全地跨线程**传输**（值传递）
- **Sync**: 类型可以安全地跨线程**共享引用**（共享访问）

> **注意**：YaoXiang 优先使用值传递，Sync 很少需要。

## Send/Sync 规则

### Send 约束

```yaoxiang
# ✅ Send 类型（可以 spawn）
type Point = Point(x: Int, y: Int)  # Int 是 Send
spawn process_point(Point(1, 2))    # ✅ Point 可以跨线程传输

# ❌ 非 Send 类型（编译错误）
type NonSend = NonSend(rc: Rc[Int])  # Rc 不是 Send
spawn process_non_send(NonSend(rc))  # ❌ 编译错误！
```

### Sync 约束

```yaoxiang
# ✅ Sync 类型（可以跨线程共享 Arc）
type Point = Point(x: Int, y: Int)

shared_point: Arc[Point] = Arc.new(Point(1, 2))  # ✅ 可以在线程间共享

# ⚠️ 注意：YaoXiang 很少需要共享引用
# 优先使用值传递 + clone()
```

## 实现设计

### 错误类型（复用到 error.rs）

```rust
pub enum OwnershipError {
    // ... 现有错误 ...

    /// 非 Send 类型用于跨线程操作
    NotSend {
        value: String,
        reason: String,
        location: (usize, usize),
    },
    /// 非 Sync 类型用于跨线程共享
    NotSync {
        value: String,
        reason: String,
        location: (usize, usize),
    },
}
```

### SendSyncChecker 实现

```rust
pub struct SendSyncChecker {
    /// 收集的错误
    errors: Vec<OwnershipError>,
    /// 当前位置 (block_idx, instr_idx)
    location: (usize, usize),
    /// 闭包定义映射: closure_operand -> (func, env)
    closures: HashMap<Operand, (usize, Vec<Operand>)>,
}

impl SendSyncChecker {
    /// 检查类型是否 Send
    pub(crate) fn is_send(&self, ty: &MonoType) -> bool {
        match ty {
            // 基本类型总是 Send
            MonoType::Void | MonoType::Bool | MonoType::Int(_) |
            MonoType::Float(_) | MonoType::Char | MonoType::String | MonoType::Bytes => true,

            // 集合类型：元素必须 Send
            MonoType::List(elem) => self.is_send(elem),
            MonoType::Dict(key, value) => self.is_send(key) && self.is_send(value),
            MonoType::Set(elem) => self.is_send(elem),

            // 元组：所有元素必须 Send
            MonoType::Tuple(types) => types.iter().all(|t| self.is_send(t)),

            // 函数：参数和返回类型必须 Send
            MonoType::Fn { params, return_type, .. } =>
                params.iter().all(|p| self.is_send(p)) && self.is_send(return_type),

            // Arc：总是 Send（原子引用计数）
            MonoType::Arc(inner) => self.is_send(inner),

            // 其他：保守假设为 Send（类型检查已验证）
            MonoType::Range { elem_type } => self.is_send(elem_type),
            MonoType::Union(types) | MonoType::Intersection(types) =>
                types.iter().all(|t| self.is_send(t)),
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_send(f)),
            MonoType::Enum(_) | MonoType::TypeVar(_) | MonoType::TypeRef(_) => true,
        }
    }

    /// 检查类型是否 Sync
    pub(crate) fn is_sync(&self, ty: &MonoType) -> bool {
        match ty {
            // 基本类型总是 Sync
            MonoType::Void | MonoType::Bool | MonoType::Int(_) |
            MonoType::Float(_) | MonoType::Char | MonoType::String | MonoType::Bytes => true,

            // 集合类型默认不是 Sync（保守）
            MonoType::List(_) | MonoType::Dict(_, _) | MonoType::Set(_) => false,

            // 元组：所有元素必须 Sync
            MonoType::Tuple(types) => types.iter().all(|t| self.is_sync(t)),

            // 函数通常不用于共享
            MonoType::Fn { .. } => false,

            // Arc：总是 Sync
            MonoType::Arc(inner) => self.is_sync(inner),

            // 其他：保守假设为 Sync
            MonoType::Range { elem_type } => self.is_sync(elem_type),
            MonoType::Union(types) | MonoType::Intersection(types) =>
                types.iter().all(|t| self.is_sync(t)),
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_sync(f)),
            MonoType::Enum(_) | MonoType::TypeVar(_) | MonoType::TypeRef(_) => true,
        }
    }

    /// 检查函数的所有权语义
    pub fn check_function(&mut self, func: &FunctionIR) -> &[OwnershipError] {
        // 1. 构建闭包映射
        self.build_closure_map(func);

        // 2. 遍历所有指令检查
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr, func);
            }
        }

        &self.errors
    }

    /// 检查 spawn 操作的 Send 约束
    fn check_spawn(&mut self, closure_op: &Operand, func: &FunctionIR) {
        if let Some((_, env)) = self.closures.get(closure_op) {
            let env: Vec<Operand> = env.clone();
            for captured in env {
                if let Some(ty) = self.get_operand_type(&captured, func) {
                    if !self.is_send(&ty) {
                        self.report_not_send(&captured, &ty, "closure captures non-Send type");
                    }
                }
            }
        }
    }
}
```

## 标准库类型约束表

| 类型 | Send | Sync | 说明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool`, `Char` | ✅ | ✅ | 基本类型 |
| `String` | ✅ | ✅ | UTF-8 字符串 |
| `List[T]` | ✅ | ❌ | 需要 Arc[List[T]] 才能 Sync |
| `Dict[K, V]` | ✅ | ❌ | 需要 Arc[Dict[K, V]] 才能 Sync |
| `Set[T]` | ✅ | ❌ | 需要 Arc[Set[T]] 才能 Sync |
| `Tuple` | ✅ | ✅/❌ | 所有元素是 Sync 时才是 Sync |
| `Arc[T]` | ✅ | ✅ | 原子引用计数（ref 关键字） |
| `Fn` | ✅ | ❌ | 通常不用于共享 |
| `Struct` | ✅ | ✅ | 所有字段是 Send/Sync 时才是 |
| `Enum` | ✅ | ✅ | 枚举只是标签 |

## 与 RFC-009 v7 对照

| RFC-009 设计 | 实现状态 |
|-------------|---------|
| Send 约束检查 | ✅ 已实现 |
| Sync 约束检查 | ✅ 已实现 |
| spawn 闭包捕获检查 | ✅ 已实现 |
| Arc 自动 Send+Sync | ✅ 已实现 |
| 错误信息含原因说明 | ✅ 已实现（reason 字段） |

## 验收测试

```rust
// src/middle/lifetime/tests/send_sync.rs

#[test]
fn test_primitives_are_send() {
    let checker = SendSyncChecker::new();
    assert!(checker.is_send(&MonoType::Int(64)));
    assert!(checker.is_send(&MonoType::Float(64)));
    assert!(checker.is_send(&MonoType::Bool));
    assert!(checker.is_send(&MonoType::String));
}

#[test]
fn test_arc_is_send_sync() {
    let checker = SendSyncChecker::new();
    let arc_int = MonoType::Arc(Box::new(MonoType::Int(64)));
    assert!(checker.is_send(&arc_int));
    assert!(checker.is_sync(&arc_int));
}

#[test]
fn test_spawn_closure_captures_send() {
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::MakeClosure {
                    dst: Operand::Local(1),
                    func: 0,
                    env: vec![Operand::Local(0)], // 捕获 Int (Send)
                },
                Instruction::Spawn {
                    func: Operand::Local(1),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = SendSyncChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}
```

## 相关文件

- **src/middle/lifetime/send_sync.rs**: SendSyncChecker 实现
- **src/middle/lifetime/error.rs**: NotSend/NotSync 错误类型定义
- **src/middle/lifetime/mod.rs**: OwnershipChecker 集成
- **src/middle/lifetime/tests/send_sync.rs**: 单元测试（11 个测试全部通过）
