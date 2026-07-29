use crate::{
    ast::statement::Statement,
    codegen::{Codegen, Scope, ScopeKind},
    error::Result,
    interpreter::bytecode::Bytecode,
    span::{GetSpan, Span, SpanOf},
};

impl Codegen {
    fn push_scope(&mut self, kind: ScopeKind) {
        let f = self.last_frame_mut();
        f.push_scope(kind);
    }
    fn pop_scope(&mut self) -> Option<Scope> {
        self.last_frame_mut().pop_scope()
    }
    fn trunc_eval(&mut self) {
        let f = self.last_frame_mut();
        f.eval_size = 0;
        let len = f.total_size();
        self.push_bytecode(SpanOf(Span::default(), Bytecode::Truncate(len as usize)));
    }

    pub fn gen_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Declaration(decl) => {
                self.gen_decl(decl)?;
                self.last_frame_mut().eval_size = 0;
            }
            Statement::If {
                condition,
                met_block,
                else_block,
                ..
            } => {
                self.push_scope(ScopeKind::Block);

                let load_cond = self.gen_expr(condition, None)?;
                let br_index = self.bytecodes().len();
                self.push_bytecode(SpanOf(condition.span(), Bytecode::Nop));
                self.last_frame_mut().eval_size = 0;

                for stmt in &met_block.1 {
                    self.gen_statement(stmt)?;
                }

                // If else exists, add jump to skip it
                let skip_index = else_block.as_ref().map(|_| {
                    let idx = self.bytecodes().len();
                    self.push_bytecode(SpanOf(met_block.0, Bytecode::Nop));
                    idx
                });

                let br_until = self.bytecodes().len();
                self.bytecodes_mut()[br_index].1 = Bytecode::BrFalse {
                    offset: br_until as isize - br_index as isize,
                    src: load_cond,
                };
                if let Some(e) = else_block.as_ref() {
                    for stmt in &e.1 {
                        self.gen_statement(stmt)?;
                    }
                }

                // fill in skip_index if exists
                if let Some(idx) = skip_index {
                    let current = self.bytecodes().len();
                    self.bytecodes_mut()[idx].1 = Bytecode::Jump(current as isize - idx as isize);
                }

                self.pop_scope();
                self.trunc_eval();
            }
            Statement::While {
                condition, block, ..
            } => {
                self.push_scope(ScopeKind::Loop);

                let continue_at = self.bytecodes().len();

                let load_cond = self.gen_expr(condition, None)?;
                let skip_start = self.bytecodes().len();
                self.push_bytecode(SpanOf(condition.span(), Bytecode::Nop));
                self.last_frame_mut().eval_size = 0;

                for stmt in &block.1 {
                    self.gen_statement(stmt)?;
                }

                let continue_from = self.bytecodes().len();
                self.push_bytecode(SpanOf(
                    block.0,
                    Bytecode::Jump(continue_at as isize - continue_from as isize),
                ));

                let skip_until = self.bytecodes().len();
                self.bytecodes_mut()[skip_start].1 = Bytecode::BrFalse {
                    offset: skip_until as isize - skip_start as isize,
                    src: load_cond,
                };

                self.pop_scope();
                self.trunc_eval();
            }
            _ => todo!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen};

    #[test]
    fn test_while_stmt() {
        let mut parser = Parser::new(
            r#"let i = 0
            let sum = 0
            while i <= 100 do
                sum = sum + i
                i = i + 1
            end
            print(sum)
            "#
            .as_bytes(),
        );
        let mut codegen = Codegen::default();

        while let Some(stmt) = parser.next_statement().unwrap() {
            codegen.gen_statement(&stmt).unwrap();
        }

        for bc in codegen.bytecodes() {
            println!("{:?}", bc.1);
        }
    }
    #[test]
    fn test_if_stmt() {
        let mut parser = Parser::new(
            r#"if n % 6 == 0 then
                print("fizz buzz")
            else if n % 2 == 0 then
                print("fizz")
            else if n % 3 == 0 then
                print("buzz")
            else
                print()
            end"#
                .as_bytes(),
        );
        let mut codegen = Codegen::default();

        codegen
            .gen_statement(&parser.next_statement().unwrap().unwrap())
            .unwrap();

        for bc in codegen.bytecodes() {
            println!("{:?}", bc.1);
        }
    }
}
