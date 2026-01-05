//! Standard IO library

/// Print a value
pub fn print<T: std::fmt::Display>(value: T) {
    print!("{}", value);
}

/// Print a value with newline
pub fn println<T: std::fmt::Display>(value: T) {
    println!("{}", value);
}

/// Read a line from stdin
pub fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim_end().to_string()
}

/// Read entire file
pub fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Write to file
pub fn write_file(
    path: &str,
    content: &str,
) -> bool {
    std::fs::write(path, content).is_ok()
}
