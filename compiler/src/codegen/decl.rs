use std::rc::Rc;

use crate::{
    ast::{
        declaration::{Declaration, FuncDecl, FunctionBody, VarDecl},
        expression::Closure,
    },
    codegen::Codegen,
    error::Result,
    interpreter::{bytecode::Bytecode, string::ValueStr, FnBody, FnSignature},
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub fn gen_decl(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::VarDecl(decl) => self.gen_var_decl(decl)?,
            Declaration::FuncDecl(decl) => self.gen_func_decl(decl)?,
            Declaration::Expression(expr) => {
                self.gen_expr(expr)?;
                debug_assert_eq!(self.stack_size(), 1);
                self.push_bytecode(SpanOf(expr.span(), Bytecode::Dup(0)));
            }
        }
        debug_assert_eq!(self.stack_size(), 0);
        Ok(())
    }
    pub(crate) fn create_func_sig(&mut self, decl: &Closure) -> Result<FnSignature> {
        self.push_frame();

        // declare params as local variables
        for p in &decl.params.1 {
            let p_name = ValueStr::interned(&p.get_str());
            self.decl_local(p_name).unwrap();
        }
        if let Some(var) = decl.variadic.as_ref() {
            let var_name = ValueStr::interned(&var.1.get_str());
            self.decl_local(var_name).unwrap();
        }

        // write the body
        match &*decl.body {
            FunctionBody::Block(stmts) => {
                for stmt in &stmts.1 {
                    self.gen_statement(stmt)?
                }
            }
            FunctionBody::Expression(expr) => {
                self.gen_expr(expr)?;
                self.push_bytecode(SpanOf(expr.span(), Bytecode::Return));
                debug_assert_eq!(self.stack_size(), 0);
            }
        }

        // resulting frame
        let frame = self.pop_frame().unwrap();
        Ok(FnSignature {
            arity: decl.params.1.len(),
            variadic: decl.variadic.is_some(),
            upvalues: frame.upvalues.into_iter().map(|(_, loc)| loc).collect(),
            body: FnBody::Bytecode(frame.bytecodes),
        })
    }
    fn gen_func_decl(&mut self, decl: &FuncDecl) -> Result<()> {
        // pre-declare the function name to allow recursion
        // function declaration is const by default
        let name = ValueStr::interned(&decl.ident.get_str());
        let decl_id = self.decl_local(name.clone());
        if decl_id.is_none() {
            self.push_bytecode(SpanOf(
                decl.fn_keyword,
                Bytecode::GlobalDeclare(name.clone()),
            ));
        }

        let sig = self.create_func_sig(&decl.closure)?;
        self.push_bytecode(SpanOf(decl.closure.span(), Bytecode::LoadFn(Rc::new(sig))));
        debug_assert_eq!(self.stack_size(), 1);
        self.push_bytecode(SpanOf(
            decl.span(),
            match decl_id {
                Some(id) => Bytecode::StoreLocal(id),
                None => Bytecode::StoreGlobal(name.clone()),
            },
        ));

        // mark constant
        if decl_id.is_none() {
            self.push_bytecode(SpanOf(decl.fn_keyword, Bytecode::GlobalReadOnly(name)));
        }

        Ok(())
    }
    pub(crate) fn gen_var_decl(&mut self, decl: &VarDecl) -> Result<()> {
        let name = ValueStr::interned(&decl.ident.get_str());
        let decl_id = self.decl_local(name.clone());
        if decl_id.is_none() {
            self.push_bytecode(SpanOf(
                decl.keyword.0,
                Bytecode::GlobalDeclare(name.clone()),
            ));
        }

        self.gen_expr(&decl.assigner)?;
        debug_assert_eq!(self.stack_size(), 1);
        self.push_bytecode(SpanOf(
            decl.span(),
            match decl_id {
                Some(id) => Bytecode::StoreLocal(id),
                None => Bytecode::StoreGlobal(name.clone()),
            },
        ));

        // TODO: Implement compile-time constant check for local variables!
        if decl_id.is_none() {
            if &*decl.keyword.get_str() == "const" {
                self.push_bytecode(SpanOf(decl.keyword.0, Bytecode::GlobalReadOnly(name)));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        ast::Parser,
        codegen::Codegen,
        interpreter::{
            bytecode::{BinaryOp, Bytecode},
            FnBody, FnSignature,
        },
        span::{Span, SpanOf},
    };

    #[test]
    fn test_decl() {
        let mut parser = Parser::new(
            r#"
const vec2_base = {
    x: 0,
    y: 0,
    sqr_len: \self -> self.x ** 2 + self.y ** 2,
    len: \self -> sqrt(self:sqr_len()),
}
fn vector2(x, y) setbase({x, y}, vec2_base)
            "#
            .as_bytes(),
        );
        let mut codegen = Codegen::with_source(parser.source());

        while let Some(stmt) = parser.next_statement().unwrap() {
            codegen.gen_statement(&stmt).unwrap();
        }

        let expected = [
            Bytecode::GlobalDeclare("vec2_base".into()),
            Bytecode::LoadStr("x".into()),
            Bytecode::LoadNum(0.0),
            Bytecode::LoadStr("y".into()),
            Bytecode::LoadNum(0.0),
            Bytecode::LoadStr("sqr_len".into()),
            Bytecode::LoadFn(Rc::new(FnSignature {
                arity: 1,
                variadic: false,
                upvalues: vec![],
                body: FnBody::Bytecode(
                    [
                        Bytecode::LoadLocal(0),
                        Bytecode::LoadProperty("x".into()),
                        Bytecode::LoadNum(2.0),
                        Bytecode::Binary(BinaryOp::Pow),
                        Bytecode::LoadLocal(0),
                        Bytecode::LoadProperty("y".into()),
                        Bytecode::LoadNum(2.0),
                        Bytecode::Binary(BinaryOp::Pow),
                        Bytecode::Binary(BinaryOp::Add),
                        Bytecode::Return,
                    ]
                    .into_iter()
                    .map(|bc| SpanOf(Span::default(), bc))
                    .collect(),
                ),
            })),
            Bytecode::LoadStr("len".into()),
            Bytecode::LoadFn(Rc::new(FnSignature {
                arity: 1,
                variadic: false,
                upvalues: vec![],
                body: FnBody::Bytecode(
                    [
                        Bytecode::LoadGlobal("sqrt".into()),
                        Bytecode::LoadLocal(0),
                        Bytecode::LoadMethod("sqr_len".into()),
                        Bytecode::Call(1),
                        Bytecode::Call(0),
                        Bytecode::Return,
                    ]
                    .into_iter()
                    .map(|bc| SpanOf(Span::default(), bc))
                    .collect(),
                ),
            })),
            Bytecode::StackToObj(0),
            Bytecode::StoreGlobal("vec2_base".into()),
            Bytecode::GlobalReadOnly("vec2_base".into()),
            Bytecode::GlobalDeclare("vector2".into()),
            Bytecode::LoadFn(Rc::new(FnSignature {
                arity: 2,
                variadic: false,
                upvalues: vec![],
                body: FnBody::Bytecode(
                    [
                        Bytecode::LoadGlobal("setbase".into()),
                        Bytecode::LoadStr("x".into()),
                        Bytecode::LoadLocal(0),
                        Bytecode::LoadStr("y".into()),
                        Bytecode::LoadLocal(1),
                        Bytecode::StackToObj(1),
                        Bytecode::LoadGlobal("vec2_base".into()),
                        Bytecode::Call(0),
                        Bytecode::Return,
                    ]
                    .into_iter()
                    .map(|bc| SpanOf(Span::default(), bc))
                    .collect(),
                ),
            })),
            Bytecode::StoreGlobal("vector2".into()),
            Bytecode::GlobalReadOnly("vector2".into()),
        ];

        for (bc, expected) in codegen.bytecodes().iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(format!("{:?}", expected), format!("{:?}", bc.1));
        }
    }
}
