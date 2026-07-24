use crate::{
    ast::statement::Statement,
    codegen::Codegen,
    error::Result,
    interpreter::bytecode::Bytecode,
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub fn gen_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Declaration(decl) => {
                self.gen_decl(decl)?;
                // expression is completed, truncate every used eval space
                let f = self.last_frame_mut();
                if f.eval_size != 0 {
                    let len = f.locals.len();
                    self.push_bytecode(SpanOf(decl.span(), Bytecode::Truncate(len)));
                }
                Ok(())
            }
            _ => todo!(),
        }
    }
}
