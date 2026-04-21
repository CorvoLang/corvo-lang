use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Literal(crate::type_system::Value),
    Regex(String, String), // pattern, flags
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: crate::type_system::Value,
    },
    VarGet {
        name: String,
    },
    StaticGet {
        name: String,
    },
    StringInterpolation {
        parts: Vec<Expr>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        named_args: HashMap<String, Expr>,
    },
    IndexAccess {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    SliceAccess {
        target: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    MethodCall {
        target: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        named_args: HashMap<String, Expr>,
    },
    ProcedureLiteral {
        params: Vec<String>,
        body: Vec<crate::ast::stmt::Stmt>,
    },
    SharedArg {
        name: String,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    pub fn span(&self) -> Option<Span> {
        None
    }
}
