// PL/SQL Parser Module
//
// This module provides PL/SQL parsing functionality organized into submodules:
// - `ast_nodes`: AST node definitions (structs, enums, types)
// - `lexer`: Lexical analysis and tokenization
// - `pl_sql_parser`: Main parser implementation
//
// Public API re-exports all necessary types to maintain compatibility.

pub mod ast_nodes;
pub mod lexer;
pub mod pl_sql_parser;

// Re-export all public types for backward compatibility
pub use ast_nodes::*;
pub use pl_sql_parser::PlSqlParser;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    #[test]
    fn test_parse_simple_block() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            BEGIN
                NULL;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(block.statements[0], Statement::Null));

        Ok(())
    }

    #[test]
    fn test_parse_declarations() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            DECLARE
                x INTEGER;
                y VARCHAR2(100) := 'hello';
            BEGIN
                NULL;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.declarations.len(), 2);

        Ok(())
    }

    #[test]
    fn test_parse_if_statement() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            BEGIN
                IF x > 10 THEN
                    y := 1;
                ELSIF x > 5 THEN
                    y := 2;
                ELSE
                    y := 3;
                END IF;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(block.statements[0], Statement::If { .. }));

        Ok(())
    }
}
