use crate::{ast::{
    expression::{expression_parser, Expression},
    primary::symbol_parser,
    Parser,
}, span::Span};

#[derive(Clone)]
pub enum Statement {
    Expression(Expression),
    If(IfStmt),
    While(WhileStmt),
}
#[derive(Clone)]
pub struct WhileStmt {
    pub condition: Expression,
    pub while_block: Vec<Statement>,
    pub break_block: Option<Vec<Statement>>,
}
#[derive(Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Vec<Statement>,
    pub else_block: Option<Vec<Statement>>,
}

fn keyword_parser(keyword: &'static str) -> Parser<Span<&'static str>> {
    symbol_parser(false, keyword)
}
fn statements_parser() -> Parser<Vec<Statement>> {
    todo!()
}
fn if_stmt_parser() -> Parser<IfStmt> {
    keyword_parser("if")
        .and_then(|_| expression_parser(false))
        .and_then(|condition| keyword_parser("then").map(|_| condition))
        .and_then(|condition| statements_parser().map(|then_block| (condition, then_block)))
        .and_then(|(condition, then_block)| {
            keyword_parser("end")
                .map({
                    let condition = condition.clone();
                    let then_block = then_block.clone();
                    move |_| IfStmt {
                        condition,
                        then_block,
                        else_block: None,
                    }
                })
                .or_else({let condition = condition.clone(); let then_block = then_block.clone();move |_| {
                    keyword_parser("else")
                        .and_then(|_| if_stmt_parser().map(|stmt| vec![Statement::If(stmt)]).or_else(|_| statements_parser()))
                        .and_then(|else_block| {
                            keyword_parser("end").map(|_| IfStmt {
                                condition,
                                then_block,
                                else_block: Some(else_block),
                            })
                        })
                }})
        })
}
