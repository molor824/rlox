use std::io::stdin;
use compiler::ast::scanner::Scanner;
use compiler::ast::statement::statement_parser;

fn main() {
    loop {
        let mut line = String::new();
        stdin().read_line(&mut line).unwrap();
        
        let value = statement_parser().parse(Scanner::new(line)).unwrap().1;
        println!("{}", value);
    }
}
