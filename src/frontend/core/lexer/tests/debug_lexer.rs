//! Debug tests to understand lexer behavior

#[cfg(test)]
mod debug_tests {
    use crate::frontend::core::lexer::tokenize;

    #[test]
    fn test_simple_debug() {
        let source = "List[T]";
        let tokens = tokenize(source).unwrap();

        println!("=== DEBUG: Tokens for 'List[T]' ===");
        for (i, token) in tokens.iter().enumerate() {
            println!("  {}: {:?}", i, token.kind);
        }
        println!("=== END DEBUG ===");

        // Just test that we can tokenize
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_simple_brackets() {
        let source = "[";
        let tokens = tokenize(source).unwrap();

        println!("=== DEBUG: Tokens for '[' ===");
        for (i, token) in tokens.iter().enumerate() {
            println!("  {}: {:?}", i, token.kind);
        }
        println!("=== END DEBUG ===");

        assert!(tokens.len() >= 1);
    }
}

