use compiler::ast::Parser;

fn main() {
    let stdin = std::io::stdin();
    let mut parser = Parser::new(stdin.lock());

    loop {
        let result = parser.next_expression(false).unwrap();
        if let Some(r) = result {
            println!("{}", r);
        }
        if !parser.skip_seperator().unwrap() {
            break;
        }
    }
}
