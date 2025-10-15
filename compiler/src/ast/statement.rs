use crate::ast::error::Error;
use crate::ast::expression::multiline_expression_parser;
use crate::ast::primitive::ident_parser;
use crate::ast::strings_eq_parser;
use crate::{
    ast::{
        expression::{inline_expression_parser, Expression},
        primitive::skip_parser,
        Parser,
    },
    span::SpanOf,
};
use std::fmt;
use std::fmt::Formatter;
use std::rc::Rc;

#[derive(Clone)]
pub enum Statement {
    Expression(Expression),
    If(IfStmt),
    While(WhileStmt),
    Block(Statements),
}
impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Expression(expr) => write!(f, "$({})", expr),
            Self::If(ifstmt) => write!(f, "{}", ifstmt),
            Self::While(whilestmt) => write!(f, "{}", whilestmt),
            Self::Block(block) => write!(f, "do\n{}\nend", block.to_string_indent()),
        }
    }
}
#[derive(Clone)]
pub struct Statements(pub Vec<Statement>);
impl fmt::Display for Statements {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for stmt in self.0.iter() {
            write!(f, "\n{}", stmt)?;
        }
        Ok(())
    }
}
impl Statements {
    fn to_string_indent(&self) -> String {
        self.to_string().replace('\n', "\n.")
    }
}
#[derive(Clone)]
pub struct WhileStmt {
    pub condition: Expression,
    pub while_block: Statements,
    pub break_block: Option<Statements>,
    pub continue_block: Option<Statements>,
}
impl fmt::Display for WhileStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "$while {} do{}\n",
            self.condition,
            self.while_block.to_string_indent()
        )?;
        if let Some(break_block) = &self.break_block {
            write!(f, "$onbreak{}\n", break_block.to_string_indent())?;
        }
        if let Some(continue_block) = &self.continue_block {
            write!(f, "$oncontinue{}\n", continue_block.to_string_indent())?;
        }
        write!(f, "$end")
    }
}
#[derive(Clone)]
pub enum ElseBlock {
    Elif(Box<IfStmt>),
    Else(Statements),
}
#[derive(Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Statements,
    pub else_block: Option<ElseBlock>,
}
impl fmt::Display for IfStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "$if {} do{}\n",
            self.condition,
            self.then_block.to_string_indent()
        )?;
        match &self.else_block {
            Some(ElseBlock::Elif(if_stmt)) => write!(
                f,
                "$elif {} then{}\n",
                if_stmt.condition,
                if_stmt.then_block.to_string_indent()
            )?,
            Some(ElseBlock::Else(else_stmts)) => {
                write!(f, "$else{}\n", else_stmts.to_string_indent())?
            }
            _ => {}
        }
        write!(f, "$end")
    }
}

pub fn statement_parser() -> Parser<Statement> {
    skip_parser(true).and_then(|_| {
        if_stmt_parser()
            .map(Statement::If)
            .or_else(|_| while_stmt_parser().map(Statement::While))
            .or_else(|_| do_block_parser().map(Statement::Block))
            .or_else(|_| inline_expression_parser().map(Statement::Expression))
    })
}
pub fn keyword_parser(keyword: &'static str) -> Parser<SpanOf<&'static str>> {
    ident_parser(true).and_then(move |ident| {
        if &*ident.as_str() == keyword {
            Parser::new_ok(ident.0.add_value(keyword))
        } else {
            let str = ident.as_str().to_string();
            Parser::new_err(ident.0.add_value(Error::InvalidKeyword(str)))
        }
    })
}
pub fn keywords_parser(keywords: &'static [&'static str]) -> Parser<SpanOf<&'static str>> {
    ident_parser(true).and_then(move |ident| {
        if let Some(&keyword) = keywords.into_iter().find(|&&k| k == &*ident.as_str()) {
            Parser::new_ok(ident.0.add_value(keyword))
        } else {
            let str = ident.as_str().to_string();
            Parser::new_err(ident.0.add_value(Error::InvalidKeyword(str)))
        }
    })
}
fn statements_parser() -> Parser<Statements> {
    // Series of keywords that indicate the end of current statements scope
    const TERMINATORS: &[&str] = &["end", "else", "elif", "onbreak", "oncontinue"];
    fn seperator_parser() -> Parser<SpanOf<&'static str>> {
        skip_parser(false).and_then(|_| strings_eq_parser(&[";", "\n", "\r\n"]))
    }
    fn stmt_parser() -> Parser<Statement> {
        skip_parser(true).and_then(|_| {
            keywords_parser(TERMINATORS).then_or(
                |_| Parser::new_err_current(Error::Eof),
                |_| statement_parser(),
            )
        })
    }
    skip_parser(true)
        .map(|_| vec![])
        .fold(
            || {
                stmt_parser()
                    .optional()
                    .and_then(|stmt| seperator_parser().map(move |_| stmt))
            },
            |mut stmts, stmt| {
                if let Some(stmt) = stmt {
                    stmts.push(stmt);
                }
                stmts
            },
        )
        .and_then(|stmts| {
            stmt_parser()
                .map({
                    let mut stmts = stmts.clone();
                    move |stmt| {
                        stmts.push(stmt);
                        stmts
                    }
                })
                .or_else(move |_| Parser::new_ok(stmts))
        })
        .map(Statements)
}
fn do_block_parser() -> Parser<Statements> {
    keyword_parser("do")
        .and_then(|_| statements_parser())
        .and_then(|stmts| keyword_parser("end").map(move |_| stmts))
}
fn if_stmt_parser() -> Parser<IfStmt> {
    // This one ignores the starting if keyword
    // and allows to recursively join in elif case
    fn _if_stmt_parser() -> Parser<IfStmt> {
        multiline_expression_parser()
            .and_then(|condition| keyword_parser("do").map(move |_| condition))
            .and_then(|condition| statements_parser().map(move |stmts| (condition, stmts)))
            .map(|(condition, stmts)| (Rc::new(condition), Rc::new(stmts)))
            .and_then(|(condition, stmts)| {
                keyword_parser("end")
                    .map({
                        let condition = condition.clone();
                        let stmts = stmts.clone();
                        move |_| IfStmt {
                            condition: (*condition).clone(),
                            then_block: (*stmts).clone(),
                            else_block: None,
                        }
                    })
                    .or_else({
                        let condition = condition.clone();
                        let stmts = stmts.clone();
                        move |_| {
                            keyword_parser("else")
                                .and_then(|_| statements_parser())
                                .and_then(|else_stmts| {
                                    keyword_parser("end").map(move |_| IfStmt {
                                        condition: (*condition).clone(),
                                        then_block: (*stmts).clone(),
                                        else_block: Some(ElseBlock::Else(else_stmts)),
                                    })
                                })
                        }
                    })
                    .or_else(move |_| {
                        keyword_parser("elif").and_then(move |_| {
                            _if_stmt_parser().map(move |ifstmt| IfStmt {
                                condition: (*condition).clone(),
                                then_block: (*stmts).clone(),
                                else_block: Some(ElseBlock::Elif(ifstmt.into())),
                            })
                        })
                    })
            })
    }
    keyword_parser("if").and_then(|_| _if_stmt_parser())
}
fn while_stmt_parser() -> Parser<WhileStmt> {
    fn onbreak_parser() -> Parser<Statements> {
        keyword_parser("onbreak").and_then(|_| statements_parser())
    }
    fn oncontinue_parser() -> Parser<Statements> {
        keyword_parser("oncontinue").and_then(|_| statements_parser())
    }
    keyword_parser("while")
        .and_then(|_| multiline_expression_parser())
        .and_then(|condition| keyword_parser("do").map(move |_| condition))
        .and_then(|condition| statements_parser().map(move |while_block| (condition, while_block)))
        .and_then(|(condition, while_block)| {
            onbreak_parser()
                .optional()
                .map(move |break_block| (condition, while_block, break_block))
        })
        .and_then(|(condition, while_block, break_block)| {
            oncontinue_parser()
                .optional()
                .map(move |continue_block| WhileStmt {
                    condition,
                    while_block,
                    break_block,
                    continue_block,
                })
        })
        .and_then(|while_stmt| {
            if while_stmt.break_block.is_none() {
                onbreak_parser()
                    .optional()
                    .map(move |break_block| WhileStmt {
                        break_block,
                        ..while_stmt
                    })
            } else {
                Parser::new_ok(while_stmt)
            }
        })
        .and_then(|while_stmt| keyword_parser("end").map(move |_| while_stmt))
}

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn stmts_test() {
        let test = r"
        if a < b do



        a()
        b(); c()

        ;
        ;



        end
        ";
        let answer = r"$if (a)<(b) do
.$((a)())
.$((b)())
.$((c)())
$end";
        let result = statement_parser()
            .parse(Scanner::new(test.chars()))
            .unwrap()
            .1;
        assert_eq!(result.to_string(), answer);
    }
}
