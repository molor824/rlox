use std::io::stdin;
use compiler::ast::expression::expression_parser;
use compiler::ast::scanner::Scanner;

fn main() {
    let mut line = String::new();
    loop {
        line.clear();
        stdin().read_line(&mut line).unwrap();
        
        let parser = expression_parser();
        let (_, result) = parser.parse(Scanner::new(line.trim())).unwrap();
        
        println!("{}", result);
    }
}
