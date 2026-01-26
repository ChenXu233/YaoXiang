#[cfg(test)]
mod tests {
    use crate::frontend::lexer::tokenize;
    use crate::frontend::parser::ParserState;
    use crate::frontend::parser::ast::*;
    use crate::frontend::parser::stmt::*;
    use crate::tlog;
    use crate::util::i18n::MSG;

    #[test]
    fn test_parse_fn_type_unified() {
        let source = "(Int, Int) -> Int";
        let tokens = tokenize(source).unwrap();
        let mut state = ParserState::new(&tokens);
        let result = state.parse_type_anno();

        assert!(result.is_some());
        let ty = result.unwrap();

        match ty {
            Type::Fn {
                params,
                return_type,
            } => {
                assert_eq!(params.len(), 2);
                tlog!(info, MSG::ParserTestParsedParams, &format!("{:?}", params));
                tlog!(
                    info,
                    MSG::ParserTestParsedReturnType,
                    &format!("{:?}", return_type)
                );
            }
            _ => panic!("Expected Type::Fn, found {:?}", ty),
        }
    }

    #[test]
    fn test_parse_var_stmt_as_fn() {
        let source = "add: (Int, Int) -> Int = (a, b) => { return a + b }";
        let tokens = tokenize(source).unwrap();
        let mut state = ParserState::new(&tokens);
        let result = state.parse_stmt();

        assert!(result.is_some());
        let stmt = result.unwrap();

        match stmt.kind {
            StmtKind::Fn {
                name,
                type_annotation,
                params,
                ..
            } => {
                assert_eq!(name, "add");
                assert!(type_annotation.is_some());
                assert_eq!(params.len(), 2);
            }
            StmtKind::Var {
                name,
                type_annotation,
                initializer,
                ..
            } => {
                tlog!(info, MSG::ParserTestParsedAsVar);
                tlog!(info, MSG::ParserTestName, &name.to_string());
                tlog!(
                    info,
                    MSG::ParserTestAnnotation,
                    &format!("{:?}", type_annotation)
                );
                panic!("Should be parsed as Fn");
            }
            _ => panic!("Unexpected stmt kind: {:?}", stmt.kind),
        }
    }
}
