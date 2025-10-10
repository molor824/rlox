use std::io::{stdin, stdout, Write};
use std::iter::repeat;
use compiler::ast::scanner::Scanner;
use compiler::ast::statement::statement_parser;

#[derive(Clone, Default)]
struct StdinIter {
    line: String,
    offset: usize,
}
impl Iterator for StdinIter {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        if self.line.len() <= self.offset {
            print!("{}", if self.line.is_empty() {"> "} else {". "});
            stdout().flush().unwrap();
            self.line.clear();
            stdin().read_line(&mut self.line).ok()?;
            self.line.extend(repeat('\n').take(10));
            self.offset = 0;
        }
        let ch = self.line.get(self.offset..)?.chars().next()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}

fn main() {
    loop {
        let value = statement_parser().parse(Scanner::new(StdinIter::default())).unwrap().1;
        println!("{}", value);
    }
}
