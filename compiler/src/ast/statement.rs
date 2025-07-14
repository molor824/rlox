use crate::ast::expression::Expression;

pub enum Statement {
    Expression(Expression),
    If(IfStmt),
    While(WhileStmt),
}
pub struct WhileStmt {
    pub condition: Expression,
    pub while_block: Vec<Statement>,
    pub break_block: Option<Vec<Statement>>,
}
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Vec<Statement>,
    pub else_block: Option<Vec<Statement>>,
}
