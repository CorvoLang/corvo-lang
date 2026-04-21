use crate::ast::{
    AssertKind, BinaryOp, Expr, FallbackBlock, MatchArm, MatchPattern, Program, Stmt, UnaryOp,
};
use crate::lexer::token::{Token, TokenType};
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    in_prep_block: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let filtered = tokens
            .into_iter()
            .filter(|t| !matches!(t.token_type, TokenType::Comment(_)))
            .collect();
        Self {
            tokens: filtered,
            current: 0,
            in_prep_block: false,
        }
    }

    /// Tokens that may follow `@` as a variable name (`var` is a keyword but `@var` is allowed).
    fn is_at_variable_name(token_type: &TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Identifier(_)
                | TokenType::Var
                | TokenType::Static
                | TokenType::Prep
                | TokenType::Try
                | TokenType::Fallback
                | TokenType::Loop
                | TokenType::Browse
                | TokenType::AsyncBrowse
                | TokenType::Terminate
                | TokenType::DontPanic
                | TokenType::Match
                | TokenType::AssertEq
                | TokenType::AssertNeq
                | TokenType::AssertGt
                | TokenType::AssertGe
                | TokenType::AssertLt
                | TokenType::AssertLe
                | TokenType::AssertMatch
                | TokenType::Procedure
                | TokenType::Shared
                | TokenType::If
                | TokenType::Else
        )
    }

    fn parse_name_after_at(&mut self) -> CorvoResult<String> {
        let name = match &self.peek().token_type {
            TokenType::Identifier(s) => s.clone(),
            TokenType::Var => "var".to_string(),
            TokenType::Static => "static".to_string(),
            TokenType::Prep => "prep".to_string(),
            TokenType::Try => "try".to_string(),
            TokenType::Fallback => "fallback".to_string(),
            TokenType::Loop => "loop".to_string(),
            TokenType::Browse => "browse".to_string(),
            TokenType::AsyncBrowse => "async_browse".to_string(),
            TokenType::Terminate => "terminate".to_string(),
            TokenType::DontPanic => "dont_panic".to_string(),
            TokenType::Match => "match".to_string(),
            TokenType::AssertEq => "assert_eq".to_string(),
            TokenType::AssertNeq => "assert_neq".to_string(),
            TokenType::AssertGt => "assert_gt".to_string(),
            TokenType::AssertGe => "assert_ge".to_string(),
            TokenType::AssertLt => "assert_lt".to_string(),
            TokenType::AssertLe => "assert_le".to_string(),
            TokenType::AssertMatch => "assert_match".to_string(),
            TokenType::Procedure => "procedure".to_string(),
            TokenType::Shared => "shared".to_string(),
            TokenType::If => "if".to_string(),
            TokenType::Else => "else".to_string(),
            _ => return Err(self.error("Expected variable name after '@'")),
        };
        self.advance();
        Ok(name)
    }

    pub fn parse(&mut self) -> CorvoResult<Program> {
        let mut statements = Vec::new();
        let mut seen_non_prep = false;

        while !self.is_at_end() {
            self.skip_comments();
            if self.is_at_end() {
                break;
            }

            if matches!(self.peek().token_type, TokenType::Prep) {
                if seen_non_prep {
                    return Err(self.error("prep block must come before all other statements"));
                }
                let stmt = self.parse_prep()?;
                statements.push(stmt);
            } else {
                seen_non_prep = true;
                if let Some(stmt) = self.parse_statement()? {
                    statements.push(stmt);
                }
            }
        }

        Ok(Program::new(statements))
    }

    fn parse_statement(&mut self) -> CorvoResult<Option<Stmt>> {
        self.skip_comments();

        if self.is_at_end() {
            return Ok(None);
        }

        let stmt = match &self.peek().token_type {
            TokenType::Prep => {
                return Err(self.error("prep block can only appear at the top level of a program"));
            }
            TokenType::Static => self.parse_static_set()?,
            TokenType::Var => self.parse_var_set()?,
            TokenType::Try => self.parse_try_block()?,
            TokenType::Loop => self.parse_loop()?,
            TokenType::Browse => self.parse_browse()?,
            TokenType::AsyncBrowse => self.parse_async_browse()?,
            TokenType::Terminate => self.parse_terminate()?,
            TokenType::DontPanic => self.parse_dont_panic()?,
            TokenType::AssertEq
            | TokenType::AssertNeq
            | TokenType::AssertGt
            | TokenType::AssertGe
            | TokenType::AssertLt
            | TokenType::AssertLe
            | TokenType::AssertMatch => self.parse_assert()?,
            TokenType::If => self.parse_if()?,
            TokenType::At => {
                // @name = value       → VarSet shortcut
                // @name[index] = val  → VarIndexSet shortcut
                // @name++             → VarAddAssign shortcut (increment by 1)
                // @name--             → VarSubAssign shortcut (decrement by 1)
                // @name += expr       → VarAddAssign shortcut
                // @name -= expr       → VarSubAssign shortcut
                // @name               → ExprStmt (VarGet shortcut)
                let next_is_at_var_name = matches!(
                    self.tokens.get(self.current + 1).map(|t| &t.token_type),
                    Some(tt) if Self::is_at_variable_name(tt)
                );
                let is_simple_assignment = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::Equals)
                    );
                let is_index_assignment = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::LeftBracket)
                    );
                let is_increment = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::Increment)
                    );
                let is_decrement = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::Decrement)
                    );
                let is_add_assign = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::PlusEqual)
                    );
                let is_sub_assign = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::MinusEqual)
                    );
                let is_or_assign = next_is_at_var_name
                    && matches!(
                        self.tokens.get(self.current + 2).map(|t| &t.token_type),
                        Some(TokenType::OrEqual)
                    );
                if is_simple_assignment {
                    self.parse_at_var_set()?
                } else if is_index_assignment {
                    self.parse_at_index_set_or_expr()?
                } else if is_increment {
                    self.parse_at_var_increment()?
                } else if is_decrement {
                    self.parse_at_var_decrement()?
                } else if is_add_assign {
                    self.parse_at_var_add_assign()?
                } else if is_sub_assign {
                    self.parse_at_var_sub_assign()?
                } else if is_or_assign {
                    self.parse_at_var_or_assign()?
                } else {
                    self.parse_expr_statement()?
                }
            }
            _ => self.parse_expr_statement()?,
        };

        Ok(Some(stmt))
    }

    // --- Statement Parsers ---

    fn parse_prep(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'prep'
        self.consume(TokenType::LeftBrace, "Expected '{' after 'prep'")?;
        self.in_prep_block = true;
        let body = self.parse_block_body("prep block");
        self.in_prep_block = false;
        Ok(Stmt::PrepBlock { body: body? })
    }

    fn parse_static_set(&mut self) -> CorvoResult<Stmt> {
        if !self.in_prep_block {
            return Err(self.error("static.set() can only be used inside a prep block"));
        }
        self.advance(); // consume 'static'

        self.consume(TokenType::Dot, "Expected '.' after 'static'")?;
        self.consume(
            TokenType::Identifier("set".to_string()),
            "Expected 'set' after '.'",
        )?;

        self.consume(TokenType::LeftParen, "Expected '(' after 'set'")?;

        let name = self.parse_string_arg("static.set name")?;
        self.consume(TokenType::Comma, "Expected ',' after name")?;

        let value = self.parse_expression()?;

        self.consume(TokenType::RightParen, "Expected ')' after value")?;

        Ok(Stmt::StaticSet { name, value })
    }

    fn parse_var_set(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'var'

        self.consume(TokenType::Dot, "Expected '.' after 'var'")?;
        self.consume(
            TokenType::Identifier("set".to_string()),
            "Expected 'set' after '.'",
        )?;

        self.consume(TokenType::LeftParen, "Expected '(' after 'set'")?;

        let name = self.parse_string_arg("var.set name")?;
        self.consume(TokenType::Comma, "Expected ',' after name")?;

        let value = self.parse_expression()?;

        self.consume(TokenType::RightParen, "Expected ')' after value")?;

        Ok(Stmt::VarSet { name, value })
    }

    fn parse_try_block(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'try'
        self.consume(TokenType::LeftBrace, "Expected '{' after 'try'")?;

        let body = self.parse_block_body("try block")?;

        let mut fallbacks = Vec::new();

        while self.match_token(TokenType::Fallback) {
            self.consume(TokenType::LeftBrace, "Expected '{' after 'fallback'")?;
            let fb_body = self.parse_block_body("fallback block")?;
            fallbacks.push(FallbackBlock { body: fb_body });
        }

        Ok(Stmt::TryBlock { body, fallbacks })
    }

    fn parse_loop(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'loop'
        self.consume(TokenType::LeftBrace, "Expected '{' after 'loop'")?;

        let body = self.parse_block_body("loop block")?;

        Ok(Stmt::Loop { body })
    }

    fn parse_browse(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'browse'
        self.consume(TokenType::LeftParen, "Expected '(' after 'browse'")?;

        let iterable = self.parse_expression()?;

        self.consume(TokenType::Comma, "Expected ',' after iterable")?;

        self.consume(
            TokenType::At,
            "Expected '@' before key variable name in browse",
        )?;
        let key = self.parse_name_after_at()?;

        self.consume(TokenType::Comma, "Expected ',' after key name")?;

        self.consume(
            TokenType::At,
            "Expected '@' before value variable name in browse",
        )?;
        let value = self.parse_name_after_at()?;

        self.consume(TokenType::RightParen, "Expected ')' after value name")?;
        self.consume(TokenType::LeftBrace, "Expected '{' after ')'")?;

        let body = self.parse_block_body("browse block")?;

        Ok(Stmt::Browse {
            iterable,
            key,
            value,
            body,
        })
    }

    fn parse_terminate(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'terminate'
        Ok(Stmt::Terminate)
    }

    fn parse_async_browse(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'async_browse'
        self.consume(TokenType::LeftParen, "Expected '(' after 'async_browse'")?;

        // First argument: the list expression
        let list = self.parse_expression()?;
        self.consume(TokenType::Comma, "Expected ',' after list expression")?;

        // Second argument: @proc_var — the procedure variable
        self.consume(
            TokenType::At,
            "Expected '@' before procedure variable name in async_browse",
        )?;
        let proc_name = self.parse_name_after_at()?;
        self.consume(TokenType::Comma, "Expected ',' after procedure variable")?;

        // Third argument: @item_param — per-item binding name
        self.consume(
            TokenType::At,
            "Expected '@' before item binding name in async_browse",
        )?;
        let item_param = self.parse_name_after_at()?;

        // Remaining arguments: (,shared @varname)*
        let mut shared_vars = Vec::new();
        while self.match_token(TokenType::Comma) {
            self.consume(
                TokenType::Shared,
                "Expected 'shared' keyword before shared variable in async_browse",
            )?;
            self.consume(
                TokenType::At,
                "Expected '@' before shared variable name in async_browse",
            )?;
            let name = self.parse_name_after_at()?;
            shared_vars.push(name);
        }

        self.consume(
            TokenType::RightParen,
            "Expected ')' to close async_browse arguments",
        )?;

        Ok(Stmt::AsyncBrowse {
            list,
            proc_name,
            item_param,
            shared_vars,
        })
    }

    fn parse_dont_panic(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'dont_panic'
        self.consume(TokenType::LeftBrace, "Expected '{' after 'dont_panic'")?;
        let body = self.parse_block_body("dont_panic block")?;
        Ok(Stmt::DontPanic { body })
    }

    fn parse_if(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume 'if'
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after condition")?;

        self.consume(TokenType::LeftBrace, "Expected '{' before if body")?;
        let then_branch = self.parse_block_body("if body")?;

        let else_branch = if self.match_token(TokenType::Else) {
            if self.match_token(TokenType::If) {
                vec![self.parse_if()?]
            } else {
                self.consume(TokenType::LeftBrace, "Expected '{' before else body")?;
                self.parse_block_body("else body")?
            }
        } else {
            vec![]
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_at_var_set(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.consume(TokenType::Equals, "Expected '=' after variable name")?;
        let value = self.parse_expression()?;
        Ok(Stmt::VarSet { name, value })
    }

    fn parse_at_var_increment(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.advance(); // consume '++'
        Ok(Stmt::VarAddAssign {
            name,
            value: Expr::Literal {
                value: Value::Number(1.0),
            },
        })
    }

    fn parse_at_var_decrement(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.advance(); // consume '--'
        Ok(Stmt::VarSubAssign {
            name,
            value: Expr::Literal {
                value: Value::Number(1.0),
            },
        })
    }

    fn parse_at_var_add_assign(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.advance(); // consume '+='
        let value = self.parse_expression()?;
        Ok(Stmt::VarAddAssign { name, value })
    }

    fn parse_at_var_sub_assign(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.advance(); // consume '-='
        let value = self.parse_expression()?;
        Ok(Stmt::VarSubAssign { name, value })
    }

    fn parse_at_var_or_assign(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.advance(); // consume 'or='
        self.consume(TokenType::LeftParen, "Expected '(' after 'or='")?;
        let mut candidates = Vec::new();
        if !self.check(TokenType::RightParen) {
            candidates.push(self.parse_expression()?);
            while self.match_token(TokenType::Comma) {
                candidates.push(self.parse_expression()?);
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' after candidates")?;
        if candidates.is_empty() {
            return Err(self.error("'or=' requires at least one candidate"));
        }
        Ok(Stmt::VarOrAssign { name, candidates })
    }

    fn parse_at_index_set_or_expr(&mut self) -> CorvoResult<Stmt> {
        self.advance(); // consume '@'
        let name = self.parse_name_after_at()?;
        self.consume(TokenType::LeftBracket, "Expected '[' after variable name")?;

        // Detect slice syntax: [:end] or [:]
        if self.check(TokenType::Colon) {
            self.advance(); // consume ':'
            let end = if self.check(TokenType::RightBracket) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            self.consume(TokenType::RightBracket, "Expected ']' after slice")?;
            let access = Expr::SliceAccess {
                target: Box::new(Expr::VarGet { name }),
                start: None,
                end,
            };
            return Ok(Stmt::ExprStmt { expr: access });
        }

        let index_or_start = self.parse_expression()?;

        if self.match_token(TokenType::Colon) {
            // [start:end] or [start:]
            let end = if self.check(TokenType::RightBracket) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            self.consume(TokenType::RightBracket, "Expected ']' after slice")?;
            let access = Expr::SliceAccess {
                target: Box::new(Expr::VarGet { name }),
                start: Some(Box::new(index_or_start)),
                end,
            };
            return Ok(Stmt::ExprStmt { expr: access });
        }

        self.consume(TokenType::RightBracket, "Expected ']' after index")?;

        if self.match_token(TokenType::Equals) {
            // @name[index] = value  →  VarIndexSet
            let value = self.parse_expression()?;
            Ok(Stmt::VarIndexSet {
                name,
                index: index_or_start,
                value,
            })
        } else {
            // @name[index] used as an expression statement (read-only)
            let access = Expr::IndexAccess {
                target: Box::new(Expr::VarGet { name }),
                index: Box::new(index_or_start),
            };
            Ok(Stmt::ExprStmt { expr: access })
        }
    }

    fn parse_assert(&mut self) -> CorvoResult<Stmt> {
        let kind = match self.peek().token_type {
            TokenType::AssertEq => AssertKind::Eq,
            TokenType::AssertNeq => AssertKind::Neq,
            TokenType::AssertGt => AssertKind::Gt,
            TokenType::AssertGe => AssertKind::Ge,
            TokenType::AssertLt => AssertKind::Lt,
            TokenType::AssertLe => AssertKind::Le,
            TokenType::AssertMatch => AssertKind::Match,
            _ => return Err(self.error("Expected assertion")),
        };
        self.advance();

        self.consume(TokenType::LeftParen, "Expected '(' after assertion")?;

        let mut args = Vec::new();
        if !self.check(TokenType::RightParen) {
            args.push(self.parse_expression()?);
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

        Ok(Stmt::Assert { kind, args })
    }

    fn parse_expr_statement(&mut self) -> CorvoResult<Stmt> {
        let expr = self.parse_expression()?;
        Ok(Stmt::ExprStmt { expr })
    }

    fn parse_block_body(&mut self, context: &str) -> CorvoResult<Vec<Stmt>> {
        let mut body = Vec::new();
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
        }

        self.consume(
            TokenType::RightBrace,
            &format!("Expected '}}' after {}", context),
        )?;
        Ok(body)
    }

    // --- Expression Parsers ---

    fn parse_expression(&mut self) -> CorvoResult<Expr> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(TokenType::Or) {
            let rhs = self.parse_logical_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_equality()?;
        while self.match_token(TokenType::And) {
            let rhs = self.parse_equality()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_comparison()?;
        loop {
            if self.match_token(TokenType::EqualsEquals) {
                let rhs = self.parse_comparison()?;
                expr = Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::BangEqual) {
                let rhs = self.parse_comparison()?;
                expr = Expr::Binary {
                    op: BinaryOp::Neq,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_additive()?;
        loop {
            if self.match_token(TokenType::LessThan) {
                let rhs = self.parse_additive()?;
                expr = Expr::Binary {
                    op: BinaryOp::Lt,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::LessEqual) {
                let rhs = self.parse_additive()?;
                expr = Expr::Binary {
                    op: BinaryOp::Le,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::GreaterThan) {
                let rhs = self.parse_additive()?;
                expr = Expr::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::GreaterEqual) {
                let rhs = self.parse_additive()?;
                expr = Expr::Binary {
                    op: BinaryOp::Ge,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            if self.match_token(TokenType::Plus) {
                let rhs = self.parse_multiplicative()?;
                expr = Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::Minus) {
                let rhs = self.parse_multiplicative()?;
                expr = Expr::Binary {
                    op: BinaryOp::Sub,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> CorvoResult<Expr> {
        let mut expr = self.parse_unary()?;
        loop {
            if self.match_token(TokenType::Star) {
                let rhs = self.parse_unary()?;
                expr = Expr::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::Slash) {
                let rhs = self.parse_unary()?;
                expr = Expr::Binary {
                    op: BinaryOp::Div,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else if self.match_token(TokenType::Percent) {
                let rhs = self.parse_unary()?;
                expr = Expr::Binary {
                    op: BinaryOp::Mod,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> CorvoResult<Expr> {
        if self.match_token(TokenType::Minus) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(inner),
            });
        }
        if self.match_token(TokenType::Bang) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(inner),
            });
        }
        let expr = self.parse_primary()?;
        self.parse_postfix(expr)
    }

    fn parse_postfix(&mut self, expr: Expr) -> CorvoResult<Expr> {
        if self.match_token(TokenType::LeftBracket) {
            // Detect slice syntax: [:end], [:], [start:end], [start:]
            if self.check(TokenType::Colon) {
                self.advance(); // consume ':'
                let end = if self.check(TokenType::RightBracket) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.consume(TokenType::RightBracket, "Expected ']' after slice")?;
                let sliced = Expr::SliceAccess {
                    target: Box::new(expr),
                    start: None,
                    end,
                };
                return self.parse_postfix(sliced);
            }

            let index_or_start = self.parse_expression()?;

            if self.match_token(TokenType::Colon) {
                // [start:end] or [start:]
                let end = if self.check(TokenType::RightBracket) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.consume(TokenType::RightBracket, "Expected ']' after slice")?;
                let sliced = Expr::SliceAccess {
                    target: Box::new(expr),
                    start: Some(Box::new(index_or_start)),
                    end,
                };
                return self.parse_postfix(sliced);
            }

            // Regular index access [index]
            self.consume(TokenType::RightBracket, "Expected ']' after index")?;
            let indexed = Expr::IndexAccess {
                target: Box::new(expr),
                index: Box::new(index_or_start),
            };
            return self.parse_postfix(indexed);
        }

        if self.match_token(TokenType::Dot) {
            let method = self.consume_method_name()?;
            self.consume(TokenType::LeftParen, "Expected '(' after method name")?;
            let (args, named_args) = self.parse_call_args()?;
            self.consume(TokenType::RightParen, "Expected ')' after arguments")?;
            let method_call = Expr::MethodCall {
                target: Box::new(expr),
                method,
                args,
                named_args,
            };
            return self.parse_postfix(method_call);
        }

        Ok(expr)
    }

    /// Consume the next token as a method name. Identifiers and most keywords
    /// are accepted so that names like `match` can be used as method names
    /// (e.g. `@expression.match("...")`).
    fn consume_method_name(&mut self) -> CorvoResult<String> {
        let name = match &self.peek().token_type {
            TokenType::Identifier(s) => s.clone(),
            // Allow keywords that are valid method names
            TokenType::Match => "match".to_string(),
            tt => {
                return Err(self.error(format!("Expected method name, got: {}", tt)));
            }
        };
        self.advance();
        Ok(name)
    }

    fn parse_primary(&mut self) -> CorvoResult<Expr> {
        let token = self.peek().clone();

        match &token.token_type {
            TokenType::String(s) => {
                self.advance();
                Ok(Expr::Literal {
                    value: crate::type_system::Value::String(s.clone()),
                })
            }
            TokenType::StringInterpolation(parts) => {
                let parts = parts.clone();
                self.advance();
                self.parse_string_interpolation(parts)
            }
            TokenType::Number(n) => {
                self.advance();
                Ok(Expr::Literal {
                    value: crate::type_system::Value::Number(*n),
                })
            }
            TokenType::Boolean(b) => {
                self.advance();
                Ok(Expr::Literal {
                    value: crate::type_system::Value::Boolean(*b),
                })
            }
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if self.match_token(TokenType::Dot) {
                    self.parse_method_call(name)
                } else if self.match_token(TokenType::LeftParen) {
                    self.parse_function_call(name)
                } else {
                    Ok(Expr::VarGet { name })
                }
            }
            TokenType::Var => {
                self.advance();
                self.parse_var_get()
            }
            TokenType::Static => {
                self.advance();
                self.parse_static_get()
            }
            TokenType::LeftBracket => self.parse_list_literal(),
            TokenType::LeftBrace => self.parse_map_literal(),
            TokenType::LeftParen => {
                self.advance();
                let inner = self.parse_expression()?;
                self.consume(TokenType::RightParen, "Expected ')' after expression")?;
                Ok(inner)
            }
            TokenType::At => {
                self.advance(); // consume '@'
                let name = self.parse_name_after_at()?;
                Ok(Expr::VarGet { name })
            }
            TokenType::Match => {
                self.advance(); // consume 'match'
                self.parse_match_expr()
            }
            TokenType::Procedure => {
                self.advance(); // consume 'procedure'
                self.parse_procedure_literal()
            }
            TokenType::Regex(pattern, flags) => {
                let pattern = pattern.clone();
                let flags = flags.clone();
                self.advance();
                Ok(Expr::Literal {
                    value: crate::type_system::Value::Regex(pattern, flags),
                })
            }
            _ => Err(self.error(format!("Unexpected token: {}", token.token_type))),
        }
    }

    fn parse_string_interpolation(&self, parts: Vec<Token>) -> CorvoResult<Expr> {
        let mut expr_parts = Vec::new();

        for part in &parts {
            match &part.token_type {
                TokenType::String(s) => {
                    if !s.is_empty() {
                        expr_parts.push(Expr::Literal {
                            value: crate::type_system::Value::String(s.clone()),
                        });
                    }
                }
                TokenType::StringInterpolation(expr_tokens) => {
                    let mut sub_parser = Parser::new(expr_tokens.clone());
                    let expr = sub_parser.parse_expression()?;
                    expr_parts.push(expr);
                }
                _ => {}
            }
        }

        if expr_parts.len() == 1 {
            if let Expr::Literal { .. } = &expr_parts[0] {
                return Ok(expr_parts.into_iter().next().unwrap());
            }
        }

        Ok(Expr::StringInterpolation { parts: expr_parts })
    }

    fn parse_var_get(&mut self) -> CorvoResult<Expr> {
        self.consume(TokenType::Dot, "Expected '.' after 'var'")?;
        self.consume(
            TokenType::Identifier("get".to_string()),
            "Expected 'get' after 'var.'",
        )?;
        self.consume(TokenType::LeftParen, "Expected '(' after 'get'")?;

        let name = self.parse_string_arg("var.get")?;

        self.consume(TokenType::RightParen, "Expected ')' after name")?;

        Ok(Expr::VarGet { name })
    }

    fn parse_static_get(&mut self) -> CorvoResult<Expr> {
        self.consume(TokenType::Dot, "Expected '.' after 'static'")?;
        self.consume(
            TokenType::Identifier("get".to_string()),
            "Expected 'get' after 'static.'",
        )?;
        self.consume(TokenType::LeftParen, "Expected '(' after 'get'")?;

        let name = self.parse_string_arg("static.get")?;

        self.consume(TokenType::RightParen, "Expected ')' after name")?;

        Ok(Expr::StaticGet { name })
    }

    fn parse_match_expr(&mut self) -> CorvoResult<Expr> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'match'")?;
        let value = self.parse_expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after match value")?;
        self.consume(TokenType::LeftBrace, "Expected '{' after match header")?;

        let mut arms = Vec::new();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            self.skip_comments();
            if self.check(TokenType::RightBrace) || self.is_at_end() {
                break;
            }

            let pattern = self.parse_match_pattern()?;
            self.consume(TokenType::FatArrow, "Expected '=>' after match pattern")?;
            let body = self.parse_expression()?;
            // Optional trailing comma between arms
            self.match_token(TokenType::Comma);

            arms.push(MatchArm {
                pattern,
                body: Box::new(body),
            });
        }

        self.consume(TokenType::RightBrace, "Expected '}' to close match block")?;

        Ok(Expr::Match {
            value: Box::new(value),
            arms,
        })
    }

    fn parse_match_pattern(&mut self) -> CorvoResult<MatchPattern> {
        let token = self.peek().clone();
        match &token.token_type {
            TokenType::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(MatchPattern::Literal(Value::String(s)))
            }
            TokenType::Number(n) => {
                let n = *n;
                self.advance();
                Ok(MatchPattern::Literal(Value::Number(n)))
            }
            TokenType::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(MatchPattern::Literal(Value::Boolean(b)))
            }
            TokenType::Regex(pattern, flags) => {
                let pattern = pattern.clone();
                let flags = flags.clone();
                self.advance();
                Ok(MatchPattern::Regex(pattern, flags))
            }
            TokenType::Identifier(s) if s == "_" => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            _ => Err(self.error(format!(
                "Expected a match pattern (string, number, boolean, regex literal, or '_'), got: {}",
                token.token_type
            ))),
        }
    }

    fn parse_procedure_literal(&mut self) -> CorvoResult<Expr> {
        // procedure(@param1, @param2, ...) { body }
        self.consume(TokenType::LeftParen, "Expected '(' after 'procedure'")?;

        let mut params = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                self.consume(TokenType::At, "Expected '@' before parameter name")?;
                let name = self.parse_name_after_at()?;
                params.push(name);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(
            TokenType::RightParen,
            "Expected ')' after procedure parameters",
        )?;
        self.consume(TokenType::LeftBrace, "Expected '{' before procedure body")?;
        let body = self.parse_block_body("procedure body")?;

        Ok(Expr::ProcedureLiteral { params, body })
    }

    fn parse_list_literal(&mut self) -> CorvoResult<Expr> {
        self.advance(); // consume '['

        let mut items = Vec::new();

        if !self.check(TokenType::RightBracket) {
            items.push(self.parse_expression()?);
            while self.match_token(TokenType::Comma) {
                items.push(self.parse_expression()?);
            }
        }

        self.consume(TokenType::RightBracket, "Expected ']' after list literal")?;

        Ok(Expr::FunctionCall {
            name: "__list__".to_string(),
            args: items,
            named_args: HashMap::new(),
        })
    }

    fn parse_map_literal(&mut self) -> CorvoResult<Expr> {
        self.advance(); // consume '{'

        let mut items = Vec::new();

        if !self.check(TokenType::RightBrace) {
            loop {
                let key = self.parse_expression()?;
                self.consume(TokenType::Colon, "Expected ':' after map key")?;
                let value = self.parse_expression()?;
                items.push(key);
                items.push(value);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightBrace, "Expected '}' after map literal")?;

        Ok(Expr::FunctionCall {
            name: "__map__".to_string(),
            args: items,
            named_args: HashMap::new(),
        })
    }

    // --- Call Parsers ---

    fn parse_function_call(&mut self, name: String) -> CorvoResult<Expr> {
        let (args, named_args) = self.parse_call_args()?;
        self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

        Ok(Expr::FunctionCall {
            name,
            args,
            named_args,
        })
    }

    fn parse_method_call(&mut self, obj: String) -> CorvoResult<Expr> {
        let method = self.consume_method_name()?;

        self.consume(TokenType::LeftParen, "Expected '(' after method")?;

        let (args, named_args) = self.parse_call_args()?;
        self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

        let func_name = format!("{}.{}", obj, method);
        Ok(Expr::FunctionCall {
            name: func_name,
            args,
            named_args,
        })
    }

    fn parse_call_args(&mut self) -> CorvoResult<(Vec<Expr>, HashMap<String, Expr>)> {
        let mut args = Vec::new();
        let mut named_args = HashMap::new();

        if self.check(TokenType::RightParen) {
            return Ok((args, named_args));
        }

        loop {
            if self.is_named_param() {
                let param_name = match self.peek().token_type.clone() {
                    TokenType::Identifier(s) => s,
                    _ => break,
                };
                self.advance(); // consume identifier
                self.consume(TokenType::Colon, "Expected ':' after named parameter")?;
                let value = self.parse_expression()?;
                named_args.insert(param_name, value);
            } else {
                args.push(self.parse_expression()?);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok((args, named_args))
    }

    // --- Helpers ---

    fn parse_string_arg(&mut self, context: &str) -> CorvoResult<String> {
        match &self.peek().token_type {
            TokenType::String(s) => {
                let name = s.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(format!("Expected string for {}", context))),
        }
    }

    fn is_named_param(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Identifier(_))
            && self.peek_next().token_type == TokenType::Colon
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> &Token {
        if self.current + 1 < self.tokens.len() {
            &self.tokens[self.current + 1]
        } else {
            &self.tokens[self.current]
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn check(&self, token_type: TokenType) -> bool {
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(&token_type)
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> CorvoResult<()> {
        if self.check(token_type) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("{} (got {})", message, self.peek().token_type)))
        }
    }

    fn error(&self, message: impl Into<String>) -> CorvoError {
        let span = self.peek().span;
        CorvoError::parsing(message.into()).with_span(span)
    }

    fn skip_comments(&mut self) {
        while matches!(self.peek().token_type, TokenType::Comment(_)) {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_source(source: &str) -> CorvoResult<Program> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    fn parse_expect_err(source: &str) -> CorvoError {
        parse_source(source).expect_err(&format!("Expected parse error for: {}", source))
    }

    // --- Basic Statement Tests ---

    #[test]
    fn test_parse_var_set() {
        let program = parse_source(r#"var.set("x", "hello")"#).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::VarSet { name, .. } => assert_eq!(name, "x"),
            _ => panic!("Expected VarSet"),
        }
    }

    #[test]
    fn test_parse_static_set() {
        let program = parse_source(r#"prep { static.set("key", 42) }"#).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::PrepBlock { body } => {
                assert_eq!(body.len(), 1);
                match &body[0] {
                    Stmt::StaticSet { name, .. } => assert_eq!(name, "key"),
                    _ => panic!("Expected StaticSet inside PrepBlock"),
                }
            }
            _ => panic!("Expected PrepBlock"),
        }
    }

    #[test]
    fn test_parse_prep_block() {
        let program = parse_source(
            r#"
            prep {
                static.set("config", "value")
            }
            var.set("x", 1)
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 2);
        match &program.statements[0] {
            Stmt::PrepBlock { body } => assert_eq!(body.len(), 1),
            _ => panic!("Expected PrepBlock"),
        }
    }

    #[test]
    fn test_parse_prep_must_come_first() {
        let err = parse_expect_err(
            r#"
            var.set("x", 1)
            prep {
                static.set("config", "value")
            }
            "#,
        );
        let msg = format!("{}", err);
        assert!(msg.contains("prep block must come before all other statements"));
    }

    #[test]
    fn test_parse_prep_not_allowed_in_nested_block() {
        let err = parse_expect_err(
            r#"
            try {
                prep { static.set("x", 1) }
            } fallback {}
            "#,
        );
        let msg = format!("{}", err);
        assert!(msg.contains("prep block can only appear at the top level"));
    }

    #[test]
    fn test_parse_static_set_outside_prep_error() {
        let err = parse_expect_err(r#"static.set("key", 42)"#);
        let msg = format!("{}", err);
        assert!(msg.contains("static.set() can only be used inside a prep block"));
    }

    #[test]
    fn test_parse_terminate() {
        let program = parse_source("terminate").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Terminate => {}
            _ => panic!("Expected Terminate"),
        }
    }

    #[test]
    fn test_parse_multiple_statements() {
        let program = parse_source(
            r#"
            var.set("x", 1)
            var.set("y", 2)
            sys.echo(var.get("x"))
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 3);
    }

    // --- Expression Tests ---

    #[test]
    fn test_parse_var_get_expr() {
        let program = parse_source(r#"sys.echo(var.get("x"))"#).unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "sys.echo");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::VarGet { name } => assert_eq!(name, "x"),
                        _ => panic!("Expected VarGet inside sys.echo"),
                    }
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    #[test]
    fn test_parse_method_call() {
        let program = parse_source(r#"string.concat("hello", " world")"#).unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "string.concat");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    #[test]
    fn test_parse_function_with_named_args() {
        let program = parse_source(r#"http.get(url: "https://example.com")"#).unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::FunctionCall {
                    name,
                    args,
                    named_args,
                } => {
                    assert_eq!(name, "http.get");
                    assert!(args.is_empty());
                    assert!(named_args.contains_key("url"));
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    #[test]
    fn test_parse_mixed_positional_and_named_args() {
        let program = parse_source(r#"my_func(1, 2, key: "value")"#).unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::FunctionCall {
                    args, named_args, ..
                } => {
                    assert_eq!(args.len(), 2);
                    assert_eq!(named_args.len(), 1);
                    assert!(named_args.contains_key("key"));
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    // --- Block Tests ---

    #[test]
    fn test_parse_try_fallback() {
        let program = parse_source(
            r#"
            try {
                assert_eq(1, 1)
            } fallback {
                sys.echo("failed")
            }
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::TryBlock { body, fallbacks } => {
                assert_eq!(body.len(), 1);
                assert_eq!(fallbacks.len(), 1);
                assert_eq!(fallbacks[0].body.len(), 1);
            }
            _ => panic!("Expected TryBlock"),
        }
    }

    #[test]
    fn test_parse_try_multiple_fallbacks() {
        let program = parse_source(
            r#"
            try {
                assert_eq(1, 1)
            } fallback {
                assert_eq(2, 2)
            } fallback {
                sys.echo("all failed")
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::TryBlock { fallbacks, .. } => {
                assert_eq!(fallbacks.len(), 2);
            }
            _ => panic!("Expected TryBlock"),
        }
    }

    #[test]
    fn test_parse_nested_try_blocks() {
        let program = parse_source(
            r#"
            try {
                try {
                    assert_eq(1, 1)
                } fallback {
                    sys.echo("inner")
                }
            } fallback {
                sys.echo("outer")
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::TryBlock { body, fallbacks } => {
                assert_eq!(body.len(), 1);
                assert_eq!(fallbacks.len(), 1);
                match &body[0] {
                    Stmt::TryBlock { .. } => {}
                    _ => panic!("Expected nested TryBlock"),
                }
            }
            _ => panic!("Expected TryBlock"),
        }
    }

    #[test]
    fn test_parse_loop() {
        let program = parse_source(
            r#"
            loop {
                sys.echo("hello")
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::Loop { body } => {
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_parse_loop_with_multiple_stmts() {
        let program = parse_source(
            r#"
            loop {
                var.set("x", math.add(var.get("x"), 1))
                assert_eq(var.get("x"), 10)
                terminate
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::Loop { body } => {
                assert_eq!(body.len(), 3);
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_parse_browse_list() {
        let program = parse_source(
            r#"
            browse(var.get("items"), @key, @val) {
                sys.echo("hello")
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::Browse {
                key, value, body, ..
            } => {
                assert_eq!(key, "key");
                assert_eq!(value, "val");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected Browse"),
        }
    }

    #[test]
    fn test_parse_browse_with_at_var() {
        let program = parse_source(
            r#"
            browse(@my_list, @k, @v) {
                sys.echo(@v)
            }
            "#,
        )
        .unwrap();
        match &program.statements[0] {
            Stmt::Browse {
                key, value, body, ..
            } => {
                assert_eq!(key, "k");
                assert_eq!(value, "v");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected Browse"),
        }
    }

    // --- Assertion Tests ---

    #[test]
    fn test_parse_assert_eq() {
        let program = parse_source("assert_eq(1, 2)").unwrap();
        match &program.statements[0] {
            Stmt::Assert { kind, args } => {
                assert_eq!(*kind, AssertKind::Eq);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Assert"),
        }
    }

    #[test]
    fn test_parse_all_assertions() {
        let assertions = [
            ("assert_eq(1, 1)", AssertKind::Eq),
            ("assert_neq(1, 2)", AssertKind::Neq),
            ("assert_gt(2, 1)", AssertKind::Gt),
            ("assert_lt(1, 2)", AssertKind::Lt),
            ("assert_match(\".*\", \"test\")", AssertKind::Match),
        ];

        for (source, expected_kind) in assertions {
            let program = parse_source(source).unwrap();
            match &program.statements[0] {
                Stmt::Assert { kind, .. } => assert_eq!(*kind, expected_kind),
                _ => panic!("Expected Assert for: {}", source),
            }
        }
    }

    // --- Literal Tests ---

    #[test]
    fn test_parse_empty_list() {
        let program = parse_source(r#"var.set("items", [])"#).unwrap();
        match &program.statements[0] {
            Stmt::VarSet { value, .. } => match value {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "__list__");
                    assert!(args.is_empty());
                }
                _ => panic!("Expected FunctionCall for list literal"),
            },
            _ => panic!("Expected VarSet"),
        }
    }

    #[test]
    fn test_parse_list_with_items() {
        let program = parse_source(r#"var.set("items", [1, 2, 3])"#).unwrap();
        match &program.statements[0] {
            Stmt::VarSet { value, .. } => match value {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "__list__");
                    assert_eq!(args.len(), 3);
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected VarSet"),
        }
    }

    #[test]
    fn test_parse_empty_map() {
        let program = parse_source(r#"var.set("m", {})"#).unwrap();
        match &program.statements[0] {
            Stmt::VarSet { value, .. } => match value {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "__map__");
                    assert!(args.is_empty());
                }
                _ => panic!("Expected FunctionCall for map literal"),
            },
            _ => panic!("Expected VarSet"),
        }
    }

    #[test]
    fn test_parse_map_with_entries() {
        let program = parse_source(r#"var.set("m", {"a": 1, "b": 2})"#).unwrap();
        match &program.statements[0] {
            Stmt::VarSet { value, .. } => match value {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "__map__");
                    assert_eq!(args.len(), 4); // key, value, key, value
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected VarSet"),
        }
    }

    // --- Index Access Tests ---

    #[test]
    fn test_parse_index_access() {
        let program = parse_source(r#"list.get(var.get("items"), 0)"#).unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "list.get");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    #[test]
    fn test_parse_index_access_expr() {
        let program = parse_source("my_list[0]").unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::IndexAccess { target, index } => {
                    match target.as_ref() {
                        Expr::VarGet { name } => assert_eq!(name, "my_list"),
                        _ => panic!("Expected VarGet as index target"),
                    }
                    match index.as_ref() {
                        Expr::Literal { value } => {
                            assert_eq!(*value, crate::type_system::Value::Number(0.0))
                        }
                        _ => panic!("Expected number literal as index"),
                    }
                }
                _ => panic!("Expected IndexAccess"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    #[test]
    fn test_parse_chained_index_access() {
        let program = parse_source("matrix[0][1]").unwrap();
        match &program.statements[0] {
            Stmt::ExprStmt { expr } => match expr {
                Expr::IndexAccess { target, .. } => match target.as_ref() {
                    Expr::IndexAccess { .. } => {}
                    _ => panic!("Expected chained IndexAccess"),
                },
                _ => panic!("Expected IndexAccess"),
            },
            _ => panic!("Expected ExprStmt"),
        }
    }

    // --- Comments ---

    #[test]
    fn test_parse_comments() {
        let program = parse_source(
            r#"
            # This is a comment
            var.set("x", 1)
            # Another comment
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_comment_between_statements() {
        let program = parse_source(
            r#"
            var.set("x", 1)
            # middle comment
            var.set("y", 2)
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 2);
    }

    // --- Error Tests ---

    #[test]
    fn test_parse_error_missing_paren() {
        let err = parse_expect_err(r#"var.set("x", 1"#);
        let msg = format!("{}", err);
        assert!(msg.contains("Expected ')'"));
    }

    #[test]
    fn test_parse_error_missing_brace() {
        let err = parse_expect_err("try { sys.echo(1)");
        let msg = format!("{}", err);
        assert!(msg.contains("Expected '}'"));
    }

    #[test]
    fn test_parse_error_unexpected_token() {
        let err = parse_expect_err("+");
        let msg = format!("{}", err);
        assert!(msg.contains("Unexpected token"));
    }

    #[test]
    fn test_parse_error_var_set_missing_dot() {
        let err = parse_expect_err(r#"var set("x", 1)"#);
        let msg = format!("{}", err);
        assert!(msg.contains("Expected '.'"));
    }

    #[test]
    fn test_parse_error_missing_comma() {
        let err = parse_expect_err(r#"var.set("x" 1)"#);
        let msg = format!("{}", err);
        assert!(msg.contains("Expected ','"));
    }

    #[test]
    fn test_parse_error_static_set_missing_string() {
        let err = parse_expect_err("prep { static.set(42, 1) }");
        let msg = format!("{}", err);
        assert!(msg.contains("Expected string"));
    }

    // --- Edge Cases ---

    #[test]
    fn test_parse_empty_input() {
        let program = parse_source("").unwrap();
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let program = parse_source("   \n\t  \n  ").unwrap();
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn test_parse_comments_only() {
        let program = parse_source("# just a comment\n# another").unwrap();
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn test_parse_complex_program() {
        let program = parse_source(
            r#"
            # Initialize
            prep {
                static.set("max", 10)
            }
            var.set("counter", 0)

            # Main loop
            loop {
                var.set("counter", math.add(var.get("counter"), 1))
                try {
                    assert_eq(var.get("counter"), var.get("max"))
                    terminate
                } fallback {
                    sys.echo("counting...")
                }
            }
            "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 3);
    }
}
