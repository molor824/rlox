use compiler::{ast::Parser, codegen::Codegen, interpreter::Interpreter};
use std::{env, error::Error, fs, io::BufReader, process::exit, rc::Rc};

fn print_err_exit(err: impl Error) -> ! {
    eprintln!("{err}");
    exit(1)
}

fn main() {
    let args = env::args().skip(1);
    let mut file_path = None;
    let mut debug = false;

    for arg in args {
        match arg.as_str() {
            "-d" | "--debug" => debug = true,
            _ => {
                file_path = match file_path {
                    Some(_) => {
                        eprintln!("Unrecognized command {}", arg);
                        exit(1)
                    }
                    None => Some(arg),
                };
            }
        }
    }

    let file_path = match &file_path {
        Some(f) => f,
        None => {
            println!("No file specified, defaulting to main.rlox");
            "main.rlox"
        }
    };
    let file = match fs::OpenOptions::new().read(true).open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error loading file `{}`: {}", file_path, e);
            exit(1)
        }
    };
    let mut parser = Parser::new(BufReader::new(file));
    let mut codegen = Codegen::with_source(parser.source());
    let mut interpreter = Interpreter::default();

    while let Some(statement) = parser
        .next_statement()
        .unwrap_or_else(|err| print_err_exit(err))
    {
        if debug {
            println!("{}", statement);
        }
        codegen
            .gen_statement(&statement)
            .unwrap_or_else(|err| print_err_exit(err));
    }

    if debug {
        for bc in codegen.bytecodes() {
            println!("{:?}", bc.1);
        }
    }

    let init_sig = Rc::new(codegen.pop_init_sig());
    let init_fn = Rc::new(
        interpreter
            .create_function(init_sig)
            .unwrap_or_else(|err| print_err_exit(err)),
    );
    interpreter
        .call_function_args(init_fn, std::iter::empty())
        .unwrap_or_else(|err| print_err_exit(err));
}
