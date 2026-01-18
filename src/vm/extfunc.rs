//! 外部函数注册表
//!
//! 将 Rust 标准库函数注册为 YaoXiang 可以调用的外部函数。

use crate::vm::executor::Value;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// 外部函数定义
pub struct ExternalFunction {
    /// 函数名
    pub name: &'static str,
    /// Rust 函数指针
    pub func: fn(&[Value]) -> Value,
}

/// 外部函数注册表
pub static EXTERNAL_FUNCTIONS: Lazy<ExternalFunctionRegistry> = Lazy::new(|| {
    let mut registry = ExternalFunctionRegistry::new();
    registry.init_stdlib();
    registry
});

/// 外部函数注册表类型
#[derive(Default)]
pub struct ExternalFunctionRegistry {
    functions: HashMap<&'static str, ExternalFunction>,
}

impl ExternalFunctionRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// 注册外部函数
    pub fn register(
        &mut self,
        func: ExternalFunction,
    ) {
        self.functions.insert(func.name, func);
    }

    /// 获取外部函数
    pub fn get(
        &self,
        name: &str,
    ) -> Option<&ExternalFunction> {
        self.functions.get(name)
    }

    /// 初始化所有标准库函数
    fn init_stdlib(&mut self) {
        // std::io
        self.register(ExternalFunction {
            name: "print",
            func: ext_print,
        });
        self.register(ExternalFunction {
            name: "println",
            func: ext_println,
        });
        self.register(ExternalFunction {
            name: "read_line",
            func: ext_read_line,
        });
        self.register(ExternalFunction {
            name: "read_file",
            func: ext_read_file,
        });
        self.register(ExternalFunction {
            name: "write_file",
            func: ext_write_file,
        });
    }
}

// === std::io 实现 ===

fn ext_print(args: &[Value]) -> Value {
    for arg in args {
        print_value(arg, false);
    }
    Value::Void
}

fn ext_println(args: &[Value]) -> Value {
    for arg in args {
        print_value(arg, true);
    }
    Value::Void
}

fn ext_read_line(_args: &[Value]) -> Value {
    use std::io;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    Value::String(input.trim_end().to_string())
}

fn ext_read_file(args: &[Value]) -> Value {
    if let Some(Value::String(path)) = args.first() {
        use std::fs;
        match fs::read_to_string(path) {
            Ok(content) => Value::String(content),
            Err(_) => Value::String(String::new()),
        }
    } else {
        Value::String(String::new())
    }
}

fn ext_write_file(args: &[Value]) -> Value {
    if let (Some(Value::String(path)), Some(Value::String(content))) = (args.first(), args.get(1)) {
        use std::fs;
        Value::Bool(fs::write(path, content).is_ok())
    } else {
        Value::Bool(false)
    }
}

fn print_value(
    value: &Value,
    newline: bool,
) {
    match value {
        Value::String(s) => {
            if newline {
                println!("{}", s);
            } else {
                print!("{}", s);
            }
        }
        Value::Int(n) => {
            if newline {
                println!("{}", n);
            } else {
                print!("{}", n);
            }
        }
        Value::Float(f) => {
            if newline {
                println!("{}", f);
            } else {
                print!("{}", f);
            }
        }
        Value::Bool(b) => {
            if newline {
                println!("{}", b);
            } else {
                print!("{}", b);
            }
        }
        Value::Char(c) => {
            if newline {
                println!("{}", c);
            } else {
                print!("{}", c);
            }
        }
        Value::Void => {
            if newline {
                println!("()");
            } else {
                print!("()");
            }
        }
        _ => {
            if newline {
                println!("{:?}", value);
            } else {
                print!("{:?}", value);
            }
        }
    }
}
