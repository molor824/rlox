use crate::{
    ast::declaration::VarDecl,
    codegen::Codegen,
    error::Result,
    interpreter::{bytecode::Store, string::InternedStr},
};

impl Codegen {
    pub(crate) fn gen_var_decl(&mut self, decl: &VarDecl) -> Result<()> {
        let name = InternedStr::from(&decl.ident.get_str() as &str);
        let var_store_method = match self.push_local(name) {
            Some(id) => Store::Local(id),
            None => Store::Global(name),
        };

        self.gen_expr(&decl.assigner, Some(var_store_method))?;
        Ok(())
    }
}
