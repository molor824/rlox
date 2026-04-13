use compiler::ast::Parser;

fn main() {
    let stdin = std::io::stdin();
    let mut parser = Parser::new(stdin.lock());

    loop {
        let Some(result) = parser.next_expression(true).unwrap() else {
            break;
        };
        println!("{}", result);
    }
}
