#[test]
fn test_debug_parse() {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    
    let tokens = tokenize("add: (a: Int, b: Int) -> Int = (a, b) => a + b").unwrap();
    let module = parse(&tokens).unwrap();
    println!("{:#?}", module.items[0]);
}
