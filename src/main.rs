use compiler::ast::expression::inline_expression_parser;
use compiler::ast::scanner::Scanner;
use std::io::stdin;

fn main() {
    let mut line = String::new();
    loop {
        line.clear();
        stdin().read_line(&mut line).unwrap();

        let parser = inline_expression_parser();
        let (_, result) = parser.parse(Scanner::new(line.trim())).unwrap();

        println!("{}", result);
    }
}
