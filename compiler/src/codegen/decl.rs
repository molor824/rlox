use std::rc::Rc;

use crate::{
    ast::{
        declaration::{Declaration, FuncDecl, FunctionBody, VarDecl},
        expression::Closure,
    },
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::ValueStr,
        FnBody, FnSignature,
    },
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub fn gen_decl(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::VarDecl(decl) => self.gen_var_decl(&decl),
            Declaration::FuncDecl(decl) => self.gen_func_decl(&decl),
            Declaration::Expression(expr) => self.gen_expr(expr, Some(Store::Nil)).map(|_| ()),
        }
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
                let load = self.gen_expr(expr, None)?;
                self.push_bytecode(SpanOf(expr.span(), Bytecode::Return(load)));
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
        let store_method = match self.decl_local(name.clone()) {
            Some(id) => Store::Local(id),
            None => Store::Global(name),
        };
        if let Store::Global(name) = &store_method {
            self.push_bytecode(SpanOf(
                decl.fn_keyword,
                Bytecode::GlobalDeclare(name.clone()),
            ));
        }

        let sig = self.create_func_sig(&decl.closure)?;
        self.push_bytecode(SpanOf(
            decl.closure.span(),
            Bytecode::Move {
                dst: store_method.clone(),
                src: Load::Function(Rc::new(sig)),
            },
        ));

        // mark constant
        match store_method {
            Store::Global(name) => {
                self.push_bytecode(SpanOf(decl.span(), Bytecode::GlobalReadOnly(name)))
            }
            // TODO: Implement compile time const check for local variables!
            _ => {}
        }

        Ok(())
    }
    pub(crate) fn gen_var_decl(&mut self, decl: &VarDecl) -> Result<()> {
        let name = ValueStr::interned(&decl.ident.get_str());
        let var_store_method = match self.decl_local(name.clone()) {
            Some(id) => Store::Local(id),
            None => Store::Global(name),
        };
        if let Store::Global(name) = var_store_method.clone() {
            self.push_bytecode(SpanOf(decl.keyword.0, Bytecode::GlobalDeclare(name)));
        }

        self.gen_expr(&decl.assigner, Some(var_store_method.clone()))?;

        // TODO: Implement compile-time constant check for local variables!
        if let Store::Global(name) = var_store_method {
            if &*decl.keyword.get_str() == "const" {
                self.push_bytecode(SpanOf(decl.keyword.0, Bytecode::GlobalReadOnly(name)));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen};

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
fn vector2(x, y) base({x, y}, vec2_base)
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
}
