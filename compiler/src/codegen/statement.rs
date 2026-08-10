use crate::{
    ast::statement::Statement,
    codegen::{Codegen, ScopeKind},
    error::{Error, ErrorKind, Result},
    interpreter::bytecode::Bytecode,
    span::{GetSpan, Span, SpanOf},
};

impl Codegen {
    fn push_scope(&mut self, kind: ScopeKind) {
        let loc = self.bytecodes().len();
        let f = self.last_frame_mut();
        f.push_scope(kind, loc);
    }
    fn pop_scope(&mut self) {
        let end_loc = self.bytecodes().len();
        let scope = self.last_frame_mut().pop_scope().unwrap();

        for break_loc in &scope.break_locs {
            self.bytecodes_mut()[*break_loc].1 =
                Bytecode::Jump(end_loc as isize - *break_loc as isize);
        }
        let base_loc = scope.base_loc;
        for continue_loc in &scope.continue_locs {
            self.bytecodes_mut()[*continue_loc].1 =
                Bytecode::Jump(base_loc as isize - *continue_loc as isize);
        }

        if self.last_frame().locals.len() > scope.base_local_size {
            self.push_bytecode(SpanOf(
                Span::default(),
                Bytecode::Truncate(scope.base_local_size),
            ));
            self.last_frame_mut().locals.truncate(scope.base_local_size);
        }
    }

    pub fn gen_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Declaration(decl) => self.gen_decl(decl)?,
            Statement::If {
                condition,
                met_block,
                else_block,
                ..
            } => {
                self.push_scope(ScopeKind::Block);

                self.gen_expr(condition)?;
                debug_assert_eq!(self.stack_size(), Some(1));

                let br_index = self.bytecodes().len();
                self.push_bytecode(SpanOf(condition.span(), Bytecode::BranchIf(false, 0)));

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
                self.bytecodes_mut()[br_index].1 =
                    Bytecode::BranchIf(false, br_until as isize - br_index as isize);

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
            }
            Statement::While {
                condition, block, ..
            } => {
                self.push_scope(ScopeKind::Loop);

                let continue_at = self.bytecodes().len();

                self.gen_expr(condition)?;
                debug_assert_eq!(self.stack_size(), Some(1));

                let break_start = self.bytecodes().len();
                self.push_bytecode(SpanOf(condition.span(), Bytecode::BranchIf(false, 0)));

                for stmt in &block.1 {
                    self.gen_statement(stmt)?;
                }

                let continue_from = self.bytecodes().len();
                self.push_bytecode(SpanOf(
                    block.0,
                    Bytecode::Jump(continue_at as isize - continue_from as isize),
                ));

                let break_until = self.bytecodes().len();
                self.bytecodes_mut()[break_start].1 =
                    Bytecode::BranchIf(false, break_until as isize - break_start as isize);

                self.pop_scope();
            }
            Statement::Break(span) | Statement::Continue(span) => {
                let Some(loop_idx) = self
                    .last_frame()
                    .scopes
                    .iter()
                    .rposition(|scope| matches!(scope.kind, ScopeKind::Loop))
                else {
                    return Err(Error {
                        kind: match statement {
                            Statement::Break(..) => ErrorKind::IllegalBreak,
                            _ => ErrorKind::IllegalContinue,
                        },
                        span: *span,
                        source: self.source.clone(),
                    });
                };
                let id = self.bytecodes().len();
                self.push_bytecode(SpanOf(*span, Bytecode::Nop));
                let scope = &mut self.last_frame_mut().scopes[loop_idx];
                match statement {
                    Statement::Break(..) => scope.break_locs.push(id),
                    _ => scope.continue_locs.push(id),
                }
            }
            Statement::Return(expr) => {
                if let Some(expr) = &expr.1 {
                    self.gen_expr(expr)?;
                }
                self.push_bytecode(SpanOf(expr.0, Bytecode::Return));
            }
            _ => todo!(),
        }
        debug_assert_eq!(self.stack_size(), Some(0));
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
                let j = 30
                while true do
                    if i == j then continue end
                    if i ** 2 == j then break end
                    i = i + 1
                    j = j + 20
                end
            "#
            .as_bytes(),
        );
        let mut codegen = Codegen::with_source(parser.source());

        while let Some(stmt) = parser.next_statement().unwrap() {
            codegen.gen_statement(&stmt).unwrap();
        }

        let expected = r#"GlobalDeclare("i")
            LoadNum(0.0)
            StoreGlobal("i")
            GlobalDeclare("j")
            LoadNum(30.0)
            StoreGlobal("j")
            LoadBool(true)
            BranchIf(false, 26)
            LoadGlobal("i")
            LoadGlobal("j")
            Binary(SetEq)
            BranchIf(false, 2)
            Jump(-6)
            LoadGlobal("i")
            LoadNum(2.0)
            Binary(Pow)
            LoadGlobal("j")
            Binary(SetEq)
            BranchIf(false, 2)
            Jump(14)
            LoadGlobal("i")
            LoadNum(1.0)
            Binary(Add)
            Dup(2)
            StoreGlobal("i")
            Dup(0)
            LoadGlobal("j")
            LoadNum(20.0)
            Binary(Add)
            Dup(2)
            StoreGlobal("j")
            Dup(0)
            Jump(-26)"#
            .split("\n")
            .map(str::trim)
            .collect::<Vec<_>>();

        for (bc, expected) in codegen.bytecodes().iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(expected, format!("{:?}", bc.1));
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
        let mut codegen = Codegen::with_source(parser.source());

        codegen
            .gen_statement(&parser.next_statement().unwrap().unwrap())
            .unwrap();

        let expected = r#"LoadGlobal("n")
            LoadNum(6.0)
            Binary(Rem)
            LoadNum(0.0)
            Binary(SetEq)
            BranchIf(false, 6)
            LoadGlobal("print")
            LoadStr("fizz buzz")
            Call(0)
            Dup(0)
            Jump(26)
            LoadGlobal("n")
            LoadNum(2.0)
            Binary(Rem)
            LoadNum(0.0)
            Binary(SetEq)
            BranchIf(false, 6)
            LoadGlobal("print")
            LoadStr("fizz")
            Call(0)
            Dup(0)
            Jump(15)
            LoadGlobal("n")
            LoadNum(3.0)
            Binary(Rem)
            LoadNum(0.0)
            Binary(SetEq)
            BranchIf(false, 6)
            LoadGlobal("print")
            LoadStr("buzz")
            Call(0)
            Dup(0)
            Jump(4)
            LoadGlobal("print")
            Call(0)
            Dup(0)"#
            .split("\n")
            .map(str::trim)
            .collect::<Vec<_>>();

        for (bc, expected) in codegen.bytecodes().iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(expected, format!("{:?}", bc.1));
        }
    }
}
