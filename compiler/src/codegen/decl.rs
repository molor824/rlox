use crate::{
    ast::declaration::VarDecl, codegen::Codegen, error::Result, interpreter::string::InternedStr,
};

impl Codegen {
    pub(crate) fn gen_var_decl(&mut self, decl: &VarDecl) -> Result<()> {
        let name = InternedStr::from(&decl.ident.get_str() as &str);
        let var_store_method = self.decl_variable(name);

        self.gen_expr(&decl.assigner, Some(var_store_method))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen, error::Result};

    #[test]
    fn test_decl() -> Result<()> {
        let mut parser = Parser::new("let a = 1 - 2".as_bytes());
        let mut codegen = Codegen::default();

        Ok(())
    }
}
