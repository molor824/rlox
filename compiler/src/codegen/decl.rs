use crate::{
    ast::declaration::{Declaration, VarDecl},
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Store},
        string::InternedStr,
    },
    span::SpanOf,
};

impl Codegen {
    pub fn gen_decl(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::VarDecl(decl) => self.gen_var_decl(&decl),
            Declaration::FuncDecl(_) => todo!(),
            Declaration::Expression(expr) => self.gen_expr(expr, Some(Store::Nil)).map(|_| ()),
        }
    }
    pub(crate) fn gen_var_decl(&mut self, decl: &VarDecl) -> Result<()> {
        let name = InternedStr::from(&decl.ident.get_str() as &str);
        let var_store_method = match self.decl_local(name) {
            Some(id) => Store::Local(id),
            None => Store::Global(name),
        };
        if let Store::Global(name) = var_store_method.clone() {
            self.push_bytecode(SpanOf(decl.keyword.0, Bytecode::GlobalDeclare(name)));
        }

        self.gen_expr(&decl.assigner, Some(var_store_method.clone()))?;

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
        let mut parser = Parser::new("const a = 1 - 2".as_bytes());
        let mut codegen = Codegen::default();

        codegen
            .gen_decl(&parser.next_decl(false).unwrap().unwrap())
            .unwrap();

        for bc in codegen.bytecodes {
            println!("{:?}", bc.1);
        }
    }
}
