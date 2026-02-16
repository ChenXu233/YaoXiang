//! Standard Concurrent library (YaoXiang)

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NativeDeclaration {
    pub name: &'static str,
    pub native_name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
    pub implemented: bool,
}

pub fn native_declarations() -> Vec<NativeDeclaration> {
    vec![
        NativeDeclaration {
            name: "sleep",
            native_name: "std.concurrent.sleep",
            signature: "(millis: Int) -> Void",
            doc: "Sleep for specified milliseconds.",
            implemented: true,
        },
        NativeDeclaration {
            name: "thread_id",
            native_name: "std.concurrent.thread_id",
            signature: "() -> String",
            doc: "Get current thread ID.",
            implemented: true,
        },
        NativeDeclaration {
            name: "yield_now",
            native_name: "std.concurrent.yield_now",
            signature: "() -> Void",
            doc: "Yield execution to scheduler.",
            implemented: true,
        },
    ]
}

pub fn native_name_map() -> HashMap<String, String> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name.to_string(), d.native_name.to_string()))
        .collect()
}

pub fn implemented_native_names() -> Vec<(&'static str, &'static str)> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name, d.native_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_declarations_not_empty() {
        let decls = native_declarations();
        assert!(!decls.is_empty());
    }
}
