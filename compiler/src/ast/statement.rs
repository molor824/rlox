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
            Self::Block(block) => write!(f, "$do\n{}\n$end", block.to_string_indent()),
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
                "$elif {} do{}\n",
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
    fn seperators_parser() -> Parser<()> {
        seperator_parser()
            .map(|_| ())
            .fold(seperator_parser, |_, _| ())
    }
    fn stmt_parser() -> Parser<Statement> {
        skip_parser(true).and_then(|_| {
            keywords_parser(TERMINATORS).then_or(
                |_| Parser::new_err_current(Error::Eof),
                |_| statement_parser(),
            )
        })
    }
    Parser::new_ok(vec![])
        .fold(
            || {
                stmt_parser()
                    .optional()
                    .and_then(|stmt| seperators_parser().map(move |_| stmt))
            },
            |mut stmts, stmt| {
                if let Some(stmt) = stmt {
                    stmts.push(stmt);
                }
                stmts
            },
        )
        .and_then(|mut stmts| {
            stmt_parser().optional().map({
                move |stmt| {
                    if let Some(stmt) = stmt {
                        stmts.push(stmt);
                    }
                    stmts
                }
            })
        })
        .map(Statements)
}
fn do_block_parser() -> Parser<Statements> {
    keyword_parser("do")
        .and_then(|_| statements_parser())
        .and_then(|stmts| keyword_parser("end").map(move |_| stmts))
}
fn if_stmt_parser() -> Parser<IfStmt> {
    fn _if_parser(initial_keyword: &'static str) -> Parser<IfStmt> {
        keyword_parser(initial_keyword)
            .and_then(|_| inline_expression_parser())
            .and_then(|condition| keyword_parser("do").map(move |_| condition))
            .and_then(|condition| {
                statements_parser().map(move |then_block| (condition, then_block))
            })
            .and_then(|(condition, then_block)| {
                keyword_parser("end")
                    .map(|_| None)
                    .or_else(|_| {
                        else_parser().map(move |else_block| Some(ElseBlock::Else(else_block)))
                    })
                    .or_else(|_| {
                        _if_parser("elif")
                            .map(move |elif_stmt| Some(ElseBlock::Elif(elif_stmt.into())))
                    })
                    .map(move |else_block| IfStmt {
                        condition,
                        then_block,
                        else_block,
                    })
            })
    }
    fn else_parser() -> Parser<Statements> {
        keyword_parser("else")
            .and_then(|_| statements_parser())
            .and_then(|stmts| keyword_parser("end").map(move |_| stmts))
    }
    _if_parser("if")
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

    while a < 10 do
        a += 1
       oncontinue
    onbreak
        print(a)
        end
        end
        ";
        let answer = r"$if (a)<(b) do
.$((a)())
.$((b)())
.$((c)())
.$while (a)<(10) do
..$((a)+=(1))
.$onbreak
..$((print)(a))
.$oncontinue
.$end
$end";
        let result = statement_parser()
            .parse(Scanner::new(test.chars()))
            .unwrap()
            .1;
        assert_eq!(result.to_string(), answer);
    }
}
