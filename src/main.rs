use compiler::{ast::Parser, codegen::Codegen};

fn main() {
    let stdin = std::io::stdin();
    let mut parser = Parser::new(stdin.lock());

    loop {
        let result = parser.next_statement().unwrap();
        if let Some(r) = result {
            let mut codegen = Codegen::with_source(parser.source());
            codegen.gen_statement(&r).unwrap();
            for bc in codegen.bytecodes() {
                println!("{:?}", bc.1);
            }
        }
        if !parser.skip_seperator().unwrap() {
            break;
        }
    }
}
