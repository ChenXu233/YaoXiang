
#[test]
fn test_debug_lambda() {
    use yaoxiang::frontend::core::lexer::tokenize;
    use yaoxiang::frontend::core::parser::parse;
    use yaoxiang::frontend::core::parser::ast::{StmtKind, Expr};
    
    let source = "let f = (x) => x + 1";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    let stmt = &module.items[0];
    
    if let StmtKind::Var { initializer, .. } = &stmt.kind {
        if let Some(init) = initializer {
            if let Expr::Lambda { body, .. } = init.as_ref() {
                println!("Body stmts: {:?}", body.stmts);
                if body.stmts.len() == 1 {
                    println!("First stmt kind: {:?}", body.stmts[0].kind);
                }
            }
        }
    }
    panic!("Debug output");
}

#[test]
fn test_debug_lambda() {
    use yaoxiang::frontend::core::lexer::tokenize;
    use yaoxiang::frontend::core::parser::parse;
    use yaoxiang::frontend::core::parser::ast::{StmtKind, Expr};
    
    let source = "let f = (x) => x + 1";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    let stmt = &module.items[0];
    
    if let StmtKind::Var { initializer, .. } = &stmt.kind {
        if let Some(init) = initializer {
            if let Expr::Lambda { body, .. } = init.as_ref() {
                println!("Body stmts: {:?}", body.stmts);
                if body.stmts.len() == 1 {
                    println!("First stmt kind: {:?}", body.stmts[0].kind);
                }
            }
        }
    }
    panic!("Debug output");
}
