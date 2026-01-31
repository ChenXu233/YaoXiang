#[cfg(test)]
mod test_simple_parse {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;

    #[test]
    fn test_simple_var() {
        let input = "x = 42";

        println!("Testing input: {}", input);

        match tokenize(input) {
            Ok(tokens) => {
                println!("✓ Tokenize OK, tokens: {:?}", tokens);

                match parse(&tokens) {
                    Ok(module) => {
                        println!("✓ Parse OK! Module: {:?}", module);
                        assert_eq!(module.items.len(), 1);

                        match &module.items[0].kind {
                            crate::frontend::core::parser::ast::StmtKind::Var {
                                name,
                                initializer,
                                ..
                            } => {
                                println!(
                                    "✓ Variable declaration: name={}, has_initializer={}",
                                    name,
                                    initializer.is_some()
                                );
                            }
                            _ => panic!(
                                "Expected variable declaration, got: {:?}",
                                module.items[0].kind
                            ),
                        }
                    }
                    Err(e) => {
                        println!("✗ Parse failed: {:?}", e);
                        panic!("Parse failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Tokenize failed: {:?}", e);
                panic!("Tokenize failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_lambda_with_type_annotation() {
        let input = "add: (Int, Int) -> Int = (a, b) => a + b";

        println!("Testing input: {}", input);

        match tokenize(input) {
            Ok(tokens) => {
                println!("✓ Tokenize OK, tokens: {:?}", tokens);

                match parse(&tokens) {
                    Ok(module) => {
                        println!("✓ Parse OK! Module: {:#?}", module.items[0]);
                        assert_eq!(module.items.len(), 1);

                        match &module.items[0].kind {
                            crate::frontend::core::parser::ast::StmtKind::Fn {
                                name,
                                params,
                                ..
                            } => {
                                println!(
                                    "✓ Function definition: name={}, params={:?}",
                                    name, params
                                );
                                assert_eq!(params.len(), 2);
                                assert_eq!(params[0].name, "a");
                                assert_eq!(params[1].name, "b");
                            }
                            other => panic!("Expected function definition, got: {:?}", other),
                        }
                    }
                    Err(e) => {
                        println!("✗ Parse failed: {:?}", e);
                        panic!("Parse failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Tokenize failed: {:?}", e);
                panic!("Tokenize failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_single_param_with_parens() {
        let input = "fact: Int -> Int = (n) => n + 1";

        println!("Testing input: {}", input);

        match tokenize(input) {
            Ok(tokens) => match parse(&tokens) {
                Ok(module) => {
                    println!("✓ Parse OK! Module: {:#?}", module.items[0]);

                    match &module.items[0].kind {
                        crate::frontend::core::parser::ast::StmtKind::Fn {
                            name, params, ..
                        } => {
                            println!("✓ Function definition: name={}, params={:?}", name, params);
                            assert_eq!(params.len(), 1);
                            assert_eq!(params[0].name, "n");
                            assert!(params[0].ty.is_some(), "param should have type annotation");
                        }
                        other => panic!("Expected function definition, got: {:?}", other),
                    }
                }
                Err(e) => panic!("Parse failed: {:?}", e),
            },
            Err(e) => panic!("Tokenize failed: {:?}", e),
        }
    }

    #[test]
    fn test_conditional_in_lambda() {
        // First test without if expression
        let test1 = "max: (Int, Int) -> Int = (a, b) => a + b";
        println!("\n=== Test 1: {}", test1);
        let tokens1 = tokenize(test1).unwrap();
        let module1 = parse(&tokens1).unwrap();
        println!("✓ Parse OK! Module: {:#?}", module1.items[0]);

        // Now test with if expression
        let test2 = "max: (Int, Int) -> Int = (a, b) => if a > b { a } else { b }";
        println!("\n=== Test 2: {}", test2);
        let tokens2 = tokenize(test2).unwrap();
        match parse(&tokens2) {
            Ok(module) => {
                println!("✓ Parse OK! Module: {:#?}", module.items[0]);
            }
            Err(e) => {
                println!("✗ Parse failed: {:?}", e);
            }
        }

        // Test higher-order function type
        let test3 = "map: ((Int) -> Int, List[Int]) -> List[Int] = (f, xs) => xs";
        println!("\n=== Test 3: {}", test3);
        let tokens3 = tokenize(test3).unwrap();
        match parse(&tokens3) {
            Ok(module) => {
                println!("✓ Parse OK! Module: {:#?}", module.items[0]);
            }
            Err(e) => {
                println!("✗ Parse failed: {:?}", e);
            }
        }

        // Test incomplete lambda (should fail to parse)
        let test4 = "dec: Int -> Int = (a) => ";
        println!("\n=== Test 4 (should fail): {}", test4);
        let tokens4 = tokenize(test4).unwrap();
        match parse(&tokens4) {
            Ok(module) => {
                println!("✓ Parse OK (unexpected)! Module: {:#?}", module.items);
            }
            Err(e) => {
                println!("✗ Parse failed (expected): {:?}", e);
            }
        }

        // Test missing parameters (should fail to parse)
        let test5 = "missing_body: Int -> Int = => 42";
        println!("\n=== Test 5 (should fail): {}", test5);
        let tokens5 = tokenize(test5).unwrap();
        match parse(&tokens5) {
            Ok(module) => {
                println!("✓ Parse OK (unexpected)! Module: {:#?}", module.items);
            }
            Err(e) => {
                println!("✗ Parse failed (expected): {:?}", e);
            }
        }
    }
}
