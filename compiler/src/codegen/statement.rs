use crate::{
    ast::{declaration::Declaration, statement::Statement},
    codegen::Codegen,
    error::Result,
    interpreter::bytecode::{Bytecode, Store},
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub fn gen_decl(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::VarDecl(decl) => self.gen_var_decl(&decl),
            Declaration::FuncDecl(_) => todo!(),
            Declaration::Expression(expr) => {
                self.gen_expr(expr, Some(Store::Nil))?;
                // expression is completed, truncate every used eval space
                let f = self.last_frame_mut();
                if f.eval_size != 0 {
                    let len = f.locals.len();
                    self.push_bytecode(SpanOf(expr.span(), Bytecode::Truncate(len)));
                }
                Ok(())
            }
        }
    }
    pub fn gen_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Declaration(decl) => self.gen_decl(decl),
            _ => todo!(),
        }
    }
}
