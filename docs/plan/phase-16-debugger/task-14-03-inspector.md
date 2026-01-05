# Task 14.3: 变量检查

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

支持在运行时检查和修改变量值。

## 变量检查器

```rust
/// 变量检查器
struct VariableInspector {
    /// 当前作用域
    current_scope: Option<FrameId>,
    /// 局部变量
    locals: HashMap<String, Value>,
    /// 捕获变量
    captures: HashMap<String, Value>,
}

impl VariableInspector {
    /// 获取变量值
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.locals.get(name)
            .or(self.captures.get(name))
    }

    /// 设置变量值
    pub fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DebugError> {
        if self.locals.contains_key(name) {
            self.locals.insert(name.to_string(), value);
            Ok(())
        } else if self.captures.contains_key(name) {
            self.captures.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(DebugError::VariableNotFound(name.to_string()))
        }
    }

    /// 展开复合值
    pub fn expand(&self, value: &Value) -> ExpandedValue {
        match value {
            Value::Struct(handle) => {
                // 展开结构体字段
                ExpandedValue::Struct {
                    fields: self.get_struct_fields(handle),
                }
            }
            Value::List(handle) => {
                // 展开列表元素
                ExpandedValue::List {
                    elements: self.get_list_elements(handle),
                }
            }
            _ => ExpandedValue::Leaf(value.clone()),
        }
    }
}
```

## 相关文件

- **inspector.rs**: VariableInspector
