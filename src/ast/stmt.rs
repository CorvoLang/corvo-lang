use super::expr::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct FallbackBlock {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    PrepBlock {
        body: Vec<Stmt>,
    },
    StaticSet {
        name: String,
        value: Expr,
    },
    VarSet {
        name: String,
        value: Expr,
    },
    VarIndexSet {
        name: String,
        index: Expr,
        value: Expr,
    },
    VarAddAssign {
        name: String,
        value: Expr,
    },
    VarSubAssign {
        name: String,
        value: Expr,
    },
    VarOrAssign {
        name: String,
        candidates: Vec<Expr>,
    },
    ExprStmt {
        expr: Expr,
    },
    TryBlock {
        body: Vec<Stmt>,
        fallbacks: Vec<FallbackBlock>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    Browse {
        iterable: Expr,
        key: String,
        value: String,
        body: Vec<Stmt>,
    },
    Terminate,
    Assert {
        kind: AssertKind,
        args: Vec<Expr>,
    },
    DontPanic {
        body: Vec<Stmt>,
    },
    AsyncBrowse {
        list: Expr,
        proc_name: String,
        item_param: String,
        shared_vars: Vec<String>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssertKind {
    Eq,
    Neq,
    Gt,
    Ge,
    Lt,
    Le,
    Match,
}
