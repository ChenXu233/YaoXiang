//! Standard Network library (YaoXiang)

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
            name: "http_get",
            native_name: "std.net.http_get",
            signature: "(url: String) -> String",
            doc: "Perform HTTP GET request.",
            implemented: true,
        },
        NativeDeclaration {
            name: "http_post",
            native_name: "std.net.http_post",
            signature: "(url: String, body: String) -> String",
            doc: "Perform HTTP POST request.",
            implemented: true,
        },
        NativeDeclaration {
            name: "url_encode",
            native_name: "std.net.url_encode",
            signature: "(s: String) -> String",
            doc: "URL-encode a string.",
            implemented: true,
        },
        NativeDeclaration {
            name: "url_decode",
            native_name: "std.net.url_decode",
            signature: "(s: String) -> String",
            doc: "URL-decode a string.",
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
