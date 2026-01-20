//! 外部函数注册表
//!
//! 将 Rust 标准库函数注册为 YaoXiang 可以调用的外部函数。

use crate::runtime::value::RuntimeValue;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

/// 外部函数定义
pub struct ExternalFunction {
    /// 函数名
    pub name: &'static str,
    /// Rust 函数指针
    pub func: fn(&[RuntimeValue]) -> RuntimeValue,
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

fn ext_print(args: &[RuntimeValue]) -> RuntimeValue {
    for arg in args {
        print_value(arg, false);
    }
    RuntimeValue::Unit
}

fn ext_println(args: &[RuntimeValue]) -> RuntimeValue {
    for arg in args {
        print_value(arg, true);
    }
    RuntimeValue::Unit
}

fn ext_read_line(_args: &[RuntimeValue]) -> RuntimeValue {
    use std::io;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    RuntimeValue::String(Arc::from(input.trim_end()))
}

fn ext_read_file(args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::String(path)) = args.first() {
        use std::fs;
        match fs::read_to_string(path.as_ref()) {
            Ok(content) => RuntimeValue::String(Arc::from(content)),
            Err(_) => RuntimeValue::String(Arc::from("")),
        }
    } else {
        RuntimeValue::String(Arc::from(""))
    }
}

fn ext_write_file(args: &[RuntimeValue]) -> RuntimeValue {
    if let (Some(RuntimeValue::String(path)), Some(RuntimeValue::String(content))) =
        (args.first(), args.get(1))
    {
        use std::fs;
        RuntimeValue::Bool(fs::write(path.as_ref(), content.as_ref()).is_ok())
    } else {
        RuntimeValue::Bool(false)
    }
}

fn print_value(
    value: &RuntimeValue,
    newline: bool,
) {
    match value {
        RuntimeValue::String(s) => {
            if newline {
                println!("{}", s);
            } else {
                print!("{}", s);
            }
        }
        RuntimeValue::Int(n) => {
            if newline {
                println!("{}", n);
            } else {
                print!("{}", n);
            }
        }
        RuntimeValue::Float(f) => {
            if newline {
                println!("{}", f);
            } else {
                print!("{}", f);
            }
        }
        RuntimeValue::Bool(b) => {
            if newline {
                println!("{}", b);
            } else {
                print!("{}", b);
            }
        }
        RuntimeValue::Char(c) => {
            if let Some(ch) = char::from_u32(*c) {
                if newline {
                    println!("{}", ch);
                } else {
                    print!("{}", ch);
                }
            }
        }
        RuntimeValue::Unit => {
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
