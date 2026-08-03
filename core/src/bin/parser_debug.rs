use std::env;

use bumpalo::Bump;
use melbi_core::types::manager::TypeManager;
use melbi_core::{analyzer, parser};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let result = parser::parse(&arena, &args[1]);
    let Ok(ast) = result else {
        eprintln!("Parse Error:\n{}", result.unwrap_err());
        return Ok(());
    };
    println!("Parsed AST:\n{:#?}", ast.expr);

    let result = analyzer::analyze(type_manager, &arena, &ast, &[], &[]);
    let Ok(expr) = result else {
        eprintln!("{:?}", result.unwrap_err());
        return Ok(());
    };
    println!("Typed Expression:\n{:#?}", expr.expr);
    Ok(())
}
