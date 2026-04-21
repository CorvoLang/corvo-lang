use crate::error::{CorvoError, CorvoResult};
use crate::lexer::token::{Token, TokenType};
use crate::span::{Position, Span};

pub struct Lexer<'a> {
    _source: &'a str,
    chars: Vec<char>,
    pos: Position,
    start: Position,
    /// When true, `/` begins a `/pattern/flags` regex literal; otherwise `/` is division.
    regex_allowed: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            _source: source,
            chars: source.chars().collect(),
            pos: Position::start(),
            start: Position::start(),
            regex_allowed: true,
        }
    }

    pub fn tokenize(&mut self) -> CorvoResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.token_type == TokenType::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    pub fn next_token(&mut self) -> CorvoResult<Token> {
        if self.skip_whitespace() {
            self.regex_allowed = true;
        }
        self.start = self.pos;

        if self.is_at_end() {
            self.bump_regex_allowed(&TokenType::Eof);
            return Ok(Token::new(TokenType::Eof, Span::point(self.pos)));
        }

        let ch = self.peek();

        let token = match ch {
            '#' => self.scan_comment()?,
            '"' => self.scan_string()?,
            '0'..='9' => self.scan_number()?,
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier()?,
            _ => self.scan_operator()?,
        };

        if !matches!(token.token_type, TokenType::Comment(_)) {
            self.bump_regex_allowed(&token.token_type);
        }
        Ok(token)
    }

    fn bump_regex_allowed(&mut self, tt: &TokenType) {
        self.regex_allowed = Self::regex_allowed_after_token(tt);
    }

    fn regex_allowed_after_token(tt: &TokenType) -> bool {
        matches!(
            tt,
            TokenType::LeftParen
                | TokenType::LeftBracket
                | TokenType::LeftBrace
                | TokenType::Comma
                | TokenType::Equals
                | TokenType::FatArrow
                | TokenType::Plus
                | TokenType::Minus
                | TokenType::Star
                | TokenType::Slash
                | TokenType::PlusEqual
                | TokenType::MinusEqual
                | TokenType::OrEqual
        )
    }

    /// Skips spaces, tabs, CR, and LF. Returns whether any newline was crossed.
    fn skip_whitespace(&mut self) -> bool {
        let mut crossed_newline = false;
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    crossed_newline = true;
                    self.advance();
                }
                _ => break,
            }
        }
        crossed_newline
    }

    fn scan_comment(&mut self) -> CorvoResult<Token> {
        let start = self.pos;
        let mut text = String::new();

        self.advance(); // consume '#'

        while !self.is_at_end() && self.peek() != '\n' {
            text.push(self.advance());
        }

        Ok(Token::new(
            TokenType::Comment(text),
            Span::new(start, self.pos),
        ))
    }

    fn scan_string(&mut self) -> CorvoResult<Token> {
        let start = self.pos;
        self.advance(); // consume opening quote

        let mut parts: Vec<Token> = Vec::new();
        let mut literal = String::new();
        let mut literal_start = self.pos;
        let mut has_interpolation = false;

        while !self.is_at_end() && self.peek() != '"' {
            let ch = self.peek();

            if ch == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(CorvoError::lexing("Unterminated escape sequence")
                        .with_span(Span::new(start, self.pos)));
                }
                let escaped = self.advance();
                match escaped {
                    'n' => literal.push('\n'),
                    't' => literal.push('\t'),
                    'r' => literal.push('\r'),
                    '\\' => literal.push('\\'),
                    '"' => literal.push('"'),
                    '$' => literal.push('$'),
                    _ => {
                        literal.push('\\');
                        literal.push(escaped);
                    }
                }
            } else if ch == '$' && self.peek_next() == '{' {
                has_interpolation = true;

                // Push accumulated literal as a String token
                if !literal.is_empty() {
                    parts.push(Token::string(
                        std::mem::take(&mut literal),
                        literal_start,
                        self.pos,
                    ));
                }

                self.advance(); // consume '$'
                self.advance(); // consume '{'

                // Tokenize the expression inside ${...}
                let expr_tokens = self.scan_interpolation_expr(start)?;

                // Create a StringInterpolation sub-token for the expression
                parts.push(Token::new(
                    TokenType::StringInterpolation(expr_tokens),
                    Span::new(literal_start, self.pos),
                ));

                literal_start = self.pos;
            } else {
                literal.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(
                CorvoError::lexing("Unterminated string").with_span(Span::new(start, self.pos))
            );
        }

        self.advance(); // consume closing quote
        let end = self.pos;

        if has_interpolation {
            // Push any remaining literal text
            if !literal.is_empty() {
                parts.push(Token::string(literal, literal_start, end));
            }

            // Ensure there's at least one part
            if parts.is_empty() {
                parts.push(Token::string(String::new(), start, end));
            }

            Ok(Token::new(
                TokenType::StringInterpolation(parts),
                Span::new(start, end),
            ))
        } else {
            Ok(Token::string(literal, start, end))
        }
    }

    fn scan_interpolation_expr(&mut self, string_start: Position) -> CorvoResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut brace_depth: usize = 1;
        let mut regex_allowed = true;

        while !self.is_at_end() && brace_depth > 0 {
            if self.skip_whitespace() {
                regex_allowed = true;
            }

            if self.is_at_end() || brace_depth == 0 {
                break;
            }

            let expr_start = self.pos;
            let ch = self.peek();

            match ch {
                '"' => {
                    // String inside interpolation expression
                    let mut inner_value = String::new();
                    self.advance(); // opening quote

                    while !self.is_at_end() && self.peek() != '"' {
                        let c = self.peek();
                        if c == '\\' {
                            self.advance();
                            if !self.is_at_end() {
                                let esc = self.advance();
                                match esc {
                                    'n' => inner_value.push('\n'),
                                    't' => inner_value.push('\t'),
                                    'r' => inner_value.push('\r'),
                                    '\\' => inner_value.push('\\'),
                                    '"' => inner_value.push('"'),
                                    '$' => inner_value.push('$'),
                                    _ => {
                                        inner_value.push('\\');
                                        inner_value.push(esc);
                                    }
                                }
                            }
                        } else if c == '\n' {
                            return Err(CorvoError::lexing("Unterminated string in interpolation")
                                .with_span(Span::new(string_start, self.pos)));
                        } else {
                            inner_value.push(self.advance());
                        }
                    }

                    if self.is_at_end() {
                        return Err(CorvoError::lexing("Unterminated string in interpolation")
                            .with_span(Span::new(string_start, self.pos)));
                    }

                    self.advance(); // closing quote
                    let tok = Token::string(inner_value, expr_start, self.pos);
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '{' => {
                    brace_depth += 1;
                    self.advance();
                    let tok = Token::new(TokenType::LeftBrace, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        self.advance(); // consume closing '}'
                        break;
                    }
                    self.advance();
                    let tok = Token::new(TokenType::RightBrace, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '(' => {
                    self.advance();
                    let tok = Token::new(TokenType::LeftParen, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                ')' => {
                    self.advance();
                    let tok = Token::new(TokenType::RightParen, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '[' => {
                    self.advance();
                    let tok = Token::new(TokenType::LeftBracket, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                ']' => {
                    self.advance();
                    let tok = Token::new(TokenType::RightBracket, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                ',' => {
                    self.advance();
                    let tok = Token::new(TokenType::Comma, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '.' => {
                    self.advance();
                    let tok = Token::new(TokenType::Dot, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                ':' => {
                    self.advance();
                    let tok = Token::new(TokenType::Colon, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '+' => {
                    self.advance();
                    let tt = if self.peek() == '+' {
                        self.advance();
                        TokenType::Increment
                    } else if self.peek() == '=' {
                        self.advance();
                        TokenType::PlusEqual
                    } else {
                        TokenType::Plus
                    };
                    let tok = Token::new(tt, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '-' => {
                    self.advance();
                    let tt = if self.peek() == '-' {
                        self.advance();
                        TokenType::Decrement
                    } else if self.peek() == '=' {
                        self.advance();
                        TokenType::MinusEqual
                    } else {
                        TokenType::Minus
                    };
                    let tok = Token::new(tt, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '*' => {
                    self.advance();
                    let tok = Token::new(TokenType::Star, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '0'..='9' => {
                    let mut value = String::new();
                    while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.')
                    {
                        if self.peek() == '.' && !self.peek_next().is_ascii_digit() {
                            break;
                        }
                        value.push(self.advance());
                    }
                    let num: f64 = value.parse().map_err(|_| {
                        CorvoError::lexing(format!("Invalid number: {}", value))
                            .with_span(Span::new(expr_start, self.pos))
                    })?;
                    let tok = Token::number(num, expr_start, self.pos);
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut name = String::new();
                    while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_')
                    {
                        name.push(self.advance());
                    }

                    let token_type = match name.as_str() {
                        "prep" => TokenType::Prep,
                        "static" => TokenType::Static,
                        "var" => TokenType::Var,
                        "try" => TokenType::Try,
                        "fallback" => TokenType::Fallback,
                        "loop" => TokenType::Loop,
                        "browse" => TokenType::Browse,
                        "async_browse" => TokenType::AsyncBrowse,
                        "terminate" => TokenType::Terminate,
                        "dont_panic" => TokenType::DontPanic,
                        "match" => TokenType::Match,
                        "assert_eq" => TokenType::AssertEq,
                        "assert_neq" => TokenType::AssertNeq,
                        "assert_gt" => TokenType::AssertGt,
                        "assert_ge" => TokenType::AssertGe,
                        "assert_lt" => TokenType::AssertLt,
                        "assert_le" => TokenType::AssertLe,
                        "assert_match" => TokenType::AssertMatch,
                        "true" => TokenType::Boolean(true),
                        "false" => TokenType::Boolean(false),
                        _ => TokenType::Identifier(name),
                    };

                    let tok = Token::new(token_type, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '@' => {
                    self.advance();
                    let tok = Token::new(TokenType::At, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '=' => {
                    self.advance();
                    let token_type = if self.peek() == '>' {
                        self.advance();
                        TokenType::FatArrow
                    } else {
                        TokenType::Equals
                    };
                    let tok = Token::new(token_type, Span::new(expr_start, self.pos));
                    regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                    tokens.push(tok);
                }
                '/' => {
                    if regex_allowed {
                        self.start = self.pos;
                        self.advance(); // consume '/'
                        let tok = self.scan_regex()?;
                        regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                        tokens.push(tok);
                    } else {
                        self.advance();
                        let tok = Token::new(TokenType::Slash, Span::new(expr_start, self.pos));
                        regex_allowed = Self::regex_allowed_after_token(&tok.token_type);
                        tokens.push(tok);
                    }
                }
                _ => {
                    return Err(CorvoError::lexing(format!(
                        "Unexpected character '{}' in interpolation expression",
                        ch
                    ))
                    .with_span(Span::new(expr_start, self.pos)));
                }
            }
        }

        if brace_depth > 0 {
            return Err(CorvoError::lexing("Unterminated interpolation expression")
                .with_span(Span::new(string_start, self.pos)));
        }

        // Add EOF to mark end of expression tokens
        tokens.push(Token::new(TokenType::Eof, Span::point(self.pos)));

        Ok(tokens)
    }

    fn scan_regex(&mut self) -> CorvoResult<Token> {
        // The opening '/' has already been consumed by the caller.
        // self.start holds the position of the '/' for error spans.
        let start = self.start;
        let mut pattern = String::new();

        while !self.is_at_end() && self.peek() != '/' {
            let ch = self.peek();
            if ch == '\\' {
                pattern.push(self.advance()); // consume '\'
                if !self.is_at_end() {
                    pattern.push(self.advance()); // consume escaped char
                }
            } else if ch == '\n' {
                return Err(CorvoError::lexing("Unterminated regex literal")
                    .with_span(Span::new(start, self.pos)));
            } else {
                pattern.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(CorvoError::lexing("Unterminated regex literal")
                .with_span(Span::new(start, self.pos)));
        }

        self.advance(); // consume closing '/'

        // Scan optional flags (letters only)
        let mut flags = String::new();
        while !self.is_at_end() && self.peek().is_ascii_alphabetic() {
            flags.push(self.advance());
        }

        let end = self.pos;
        Ok(Token::new(
            TokenType::Regex(pattern, flags),
            Span::new(start, end),
        ))
    }

    fn scan_number(&mut self) -> CorvoResult<Token> {
        let start = self.pos;
        let mut value = String::new();

        if self.peek() == '0' {
            let next = self.peek_next();
            if next == 'x' || next == 'X' {
                self.advance(); // consume '0'
                self.advance(); // consume 'x'
                let mut hex = String::new();
                while !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                    hex.push(self.advance());
                }
                let num = u64::from_str_radix(&hex, 16).map_err(|_| {
                    CorvoError::lexing(format!("Invalid hex number: 0x{}", hex))
                })?;
                return Ok(Token::number(num as f64, start, self.pos));
            } else if next == 'o' || next == 'O' {
                self.advance(); // consume '0'
                self.advance(); // consume 'o'
                let mut octal = String::new();
                while !self.is_at_end()
                    && self.peek().is_ascii_digit()
                    && self.peek() != '8'
                    && self.peek() != '9'
                {
                    octal.push(self.advance());
                }
                let num = u64::from_str_radix(&octal, 8).map_err(|_| {
                    CorvoError::lexing(format!("Invalid octal number: 0o{}", octal))
                })?;
                return Ok(Token::number(num as f64, start, self.pos));
            }
        }

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            if self.peek() == '.' && !self.peek_next().is_ascii_digit() {
                break;
            }
            value.push(self.advance());
        }

        let num: f64 = value.parse().map_err(|_| {
            CorvoError::lexing(format!("Invalid number: {}", value))
                .with_span(Span::new(start, self.pos))
        })?;

        Ok(Token::number(num, start, self.pos))
    }

    fn scan_identifier(&mut self) -> CorvoResult<Token> {
        let start = self.pos;
        let mut name = String::new();

        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            name.push(self.advance());
        }

        let end = self.pos;

        let keyword = match name.as_str() {
            "or" if !self.is_at_end() && self.peek() == '=' => {
                self.advance(); // consume '='
                TokenType::OrEqual
            }
            "prep" => TokenType::Prep,
            "static" => TokenType::Static,
            "var" => TokenType::Var,
            "try" => TokenType::Try,
            "fallback" => TokenType::Fallback,
            "loop" => TokenType::Loop,
            "browse" => TokenType::Browse,
            "async_browse" => TokenType::AsyncBrowse,
            "terminate" => TokenType::Terminate,
            "dont_panic" => TokenType::DontPanic,
            "match" => TokenType::Match,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "assert_eq" => TokenType::AssertEq,
            "assert_neq" => TokenType::AssertNeq,
            "assert_gt" => TokenType::AssertGt,
            "assert_ge" => TokenType::AssertGe,
            "assert_lt" => TokenType::AssertLt,
            "assert_le" => TokenType::AssertLe,
            "assert_match" => TokenType::AssertMatch,
            "procedure" => TokenType::Procedure,
            "shared" => TokenType::Shared,
            "true" => TokenType::Boolean(true),
            "false" => TokenType::Boolean(false),
            _ => TokenType::Identifier(name),
        };

        Ok(Token::new(keyword, Span::new(start, end)))
    }

    fn scan_operator(&mut self) -> CorvoResult<Token> {
        let start = self.pos;
        let ch = self.advance();

        let token_type = match ch {
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            ':' => TokenType::Colon,
            '@' => TokenType::At,
            '=' => {
                if self.peek() == '>' {
                    self.advance(); // consume '>'
                    TokenType::FatArrow
                } else if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::EqualsEquals
                } else {
                    TokenType::Equals
                }
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::LessEqual
                } else {
                    TokenType::LessThan
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::GreaterEqual
                } else {
                    TokenType::GreaterThan
                }
            }
            '&' => {
                if self.peek() == '&' {
                    self.advance(); // consume '&'
                    TokenType::And
                } else {
                    TokenType::Illegal(ch.to_string())
                }
            }
            '|' => {
                if self.peek() == '|' {
                    self.advance(); // consume '|'
                    TokenType::Or
                } else if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::OrEqual
                } else {
                    TokenType::Illegal(ch.to_string())
                }
            }
            '%' => TokenType::Percent,
            '+' => {
                if self.peek() == '+' {
                    self.advance(); // consume second '+'
                    TokenType::Increment
                } else if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::PlusEqual
                } else {
                    TokenType::Plus
                }
            }
            '-' => {
                if self.peek() == '-' {
                    self.advance(); // consume second '-'
                    TokenType::Decrement
                } else if self.peek() == '=' {
                    self.advance(); // consume '='
                    TokenType::MinusEqual
                } else {
                    TokenType::Minus
                }
            }
            '*' => TokenType::Star,
            '/' => {
                if self.regex_allowed {
                    // Regex literal: /pattern/flags
                    self.start = start;
                    return self.scan_regex();
                }
                TokenType::Slash
            }
            _ => TokenType::Illegal(ch.to_string()),
        };

        let end = self.pos;
        Ok(Token::new(token_type, Span::new(start, end)))
    }

    fn is_at_end(&self) -> bool {
        self.pos.offset >= self.chars.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.pos.offset]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos.offset + 1 >= self.chars.len() {
            '\0'
        } else {
            self.chars[self.pos.offset + 1]
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.peek();
        self.pos.advance(ch);
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> CorvoResult<Vec<Token>> {
        Lexer::new(source).tokenize()
    }

    fn assert_token_types(source: &str, expected: &[TokenType]) {
        let tokens = tokenize(source).unwrap();
        let types: Vec<&TokenType> = tokens.iter().map(|t| &t.token_type).collect();
        let expected_refs: Vec<&TokenType> = expected.iter().collect();
        assert_eq!(types, expected_refs, "Token types mismatch for: {}", source);
    }

    // --- String Tests ---

    #[test]
    fn test_scan_string() {
        let tokens = tokenize("\"hello\"").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::String("hello".to_string()));
    }

    #[test]
    fn test_scan_empty_string() {
        let tokens = tokenize("\"\"").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::String("".to_string()));
    }

    #[test]
    fn test_scan_string_escape_newline() {
        let tokens = tokenize(r#""hello\nworld""#).unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_scan_string_interpolation_simple() {
        let tokens = tokenize("\"Hello ${name}\"").unwrap();
        assert_eq!(tokens.len(), 2); // StringInterpolation + Eof
        match &tokens[0].token_type {
            TokenType::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 2); // "Hello " + ${name}
                assert_eq!(parts[0].token_type, TokenType::String("Hello ".to_string()));
                match &parts[1].token_type {
                    TokenType::StringInterpolation(expr_tokens) => {
                        assert_eq!(
                            expr_tokens[0].token_type,
                            TokenType::Identifier("name".to_string())
                        );
                    }
                    _ => panic!("Expected StringInterpolation for expression part"),
                }
            }
            _ => panic!(
                "Expected StringInterpolation token, got {:?}",
                tokens[0].token_type
            ),
        }
    }

    #[test]
    fn test_scan_string_interpolation_complex_expr() {
        let tokens = tokenize(r#""Value: ${var.get("x")}""#).unwrap();
        match &tokens[0].token_type {
            TokenType::StringInterpolation(parts) => {
                assert_eq!(
                    parts[0].token_type,
                    TokenType::String("Value: ".to_string())
                );
                match &parts[1].token_type {
                    TokenType::StringInterpolation(expr_tokens) => {
                        assert_eq!(expr_tokens[0].token_type, TokenType::Var);
                        assert_eq!(expr_tokens[1].token_type, TokenType::Dot);
                        assert_eq!(
                            expr_tokens[2].token_type,
                            TokenType::Identifier("get".to_string())
                        );
                        assert_eq!(expr_tokens[3].token_type, TokenType::LeftParen);
                        assert_eq!(
                            expr_tokens[4].token_type,
                            TokenType::String("x".to_string())
                        );
                        assert_eq!(expr_tokens[5].token_type, TokenType::RightParen);
                    }
                    _ => panic!("Expected StringInterpolation for expression"),
                }
            }
            _ => panic!("Expected StringInterpolation token"),
        }
    }

    #[test]
    fn test_scan_string_interpolation_multiple() {
        let tokens = tokenize(r#""${a} and ${b}""#).unwrap();
        match &tokens[0].token_type {
            TokenType::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 3);
                match &parts[0].token_type {
                    TokenType::StringInterpolation(expr) => {
                        assert_eq!(expr[0].token_type, TokenType::Identifier("a".to_string()));
                    }
                    _ => panic!("Expected interpolation for first part"),
                }
                assert_eq!(parts[1].token_type, TokenType::String(" and ".to_string()));
                match &parts[2].token_type {
                    TokenType::StringInterpolation(expr) => {
                        assert_eq!(expr[0].token_type, TokenType::Identifier("b".to_string()));
                    }
                    _ => panic!("Expected interpolation for third part"),
                }
            }
            _ => panic!("Expected StringInterpolation token"),
        }
    }

    #[test]
    fn test_scan_string_interpolation_math_expr() {
        let tokens = tokenize(r#""Result: ${math.add(1, 2)}""#).unwrap();
        match &tokens[0].token_type {
            TokenType::StringInterpolation(parts) => {
                assert_eq!(
                    parts[0].token_type,
                    TokenType::String("Result: ".to_string())
                );
                match &parts[1].token_type {
                    TokenType::StringInterpolation(expr_tokens) => {
                        assert_eq!(
                            expr_tokens[0].token_type,
                            TokenType::Identifier("math".to_string())
                        );
                        assert_eq!(expr_tokens[1].token_type, TokenType::Dot);
                        assert_eq!(
                            expr_tokens[2].token_type,
                            TokenType::Identifier("add".to_string())
                        );
                        assert_eq!(expr_tokens[3].token_type, TokenType::LeftParen);
                        assert_eq!(expr_tokens[4].token_type, TokenType::Number(1.0));
                        assert_eq!(expr_tokens[5].token_type, TokenType::Comma);
                        assert_eq!(expr_tokens[6].token_type, TokenType::Number(2.0));
                        assert_eq!(expr_tokens[7].token_type, TokenType::RightParen);
                    }
                    _ => panic!("Expected interpolation expression"),
                }
            }
            _ => panic!("Expected StringInterpolation"),
        }
    }

    #[test]
    fn test_scan_string_no_interpolation_plain() {
        let tokens = tokenize("\"no interpolation here\"").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("no interpolation here".to_string())
        );
    }

    #[test]
    fn test_scan_string_interpolation_escaped_dollar() {
        let tokens = tokenize(r#""price \${amount}""#).unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("price ${amount}".to_string())
        );
    }

    #[test]
    fn test_scan_string_escape_backslash() {
        let tokens = tokenize(r#""path\\to\\file""#).unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("path\\to\\file".to_string())
        );
    }

    #[test]
    fn test_scan_string_escape_quote() {
        let tokens = tokenize(r#""say \"hi\"""#).unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("say \"hi\"".to_string())
        );
    }

    #[test]
    fn test_scan_string_escape_dollar() {
        let tokens = tokenize(r#""price \$5""#).unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("price $5".to_string())
        );
    }

    #[test]
    fn test_scan_string_unterminated() {
        let result = tokenize("\"hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_string_multiline() {
        let tokens = tokenize("\"line1\nline2\"").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::String("line1\nline2".to_string())
        );
    }

    // --- Number Tests ---

    #[test]
    fn test_scan_integer() {
        let tokens = tokenize("123").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(123.0));
    }

    #[test]
    fn test_scan_float() {
        let tokens = tokenize("42.5").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(42.5));
    }

    #[test]
    fn test_scan_number_with_trailing_dot() {
        // "42." should scan as 42 followed by dot operator
        let tokens = tokenize("42.x").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(42.0));
        assert_eq!(tokens[1].token_type, TokenType::Dot);
        assert_eq!(tokens[2].token_type, TokenType::Identifier("x".to_string()));
    }

    #[test]
    fn test_scan_zero() {
        let tokens = tokenize("0").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(0.0));
    }

    #[test]
    fn test_scan_float_no_leading() {
        let tokens = tokenize("0.5").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(0.5));
    }

    // --- Keyword Tests ---

    #[test]
    fn test_scan_keywords() {
        assert_token_types(
            "static var try",
            &[
                TokenType::Static,
                TokenType::Var,
                TokenType::Try,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_all_keywords() {
        assert_token_types(
            "fallback loop browse terminate assert_eq assert_neq assert_gt assert_lt assert_match",
            &[
                TokenType::Fallback,
                TokenType::Loop,
                TokenType::Browse,
                TokenType::Terminate,
                TokenType::AssertEq,
                TokenType::AssertNeq,
                TokenType::AssertGt,
                TokenType::AssertLt,
                TokenType::AssertMatch,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_booleans() {
        assert_token_types(
            "true false",
            &[
                TokenType::Boolean(true),
                TokenType::Boolean(false),
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_keyword_not_prefix_of_identifier() {
        // "staticVar" should be an identifier, not "static" + "Var"
        let tokens = tokenize("staticVar").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Identifier("staticVar".to_string())
        );
    }

    // --- Identifier Tests ---

    #[test]
    fn test_scan_identifier() {
        let tokens = tokenize("my_var").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Identifier("my_var".to_string())
        );
    }

    #[test]
    fn test_scan_identifier_with_numbers() {
        let tokens = tokenize("var123").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Identifier("var123".to_string())
        );
    }

    #[test]
    fn test_scan_identifier_underscore_start() {
        let tokens = tokenize("_private").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Identifier("_private".to_string())
        );
    }

    // --- Operator Tests ---

    #[test]
    fn test_scan_all_operators() {
        assert_token_types(
            "{ } ( ) [ ] , . :",
            &[
                TokenType::LeftBrace,
                TokenType::RightBrace,
                TokenType::LeftParen,
                TokenType::RightParen,
                TokenType::LeftBracket,
                TokenType::RightBracket,
                TokenType::Comma,
                TokenType::Dot,
                TokenType::Colon,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_infix_arithmetic_tokens() {
        assert_token_types(
            "1 + 2 - 3 * 4 / 5",
            &[
                TokenType::Number(1.0),
                TokenType::Plus,
                TokenType::Number(2.0),
                TokenType::Minus,
                TokenType::Number(3.0),
                TokenType::Star,
                TokenType::Number(4.0),
                TokenType::Slash,
                TokenType::Number(5.0),
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_slash_is_division_after_number() {
        let tokens = tokenize("10 / 2").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number(10.0));
        assert_eq!(tokens[1].token_type, TokenType::Slash);
        assert_eq!(tokens[2].token_type, TokenType::Number(2.0));
    }

    #[test]
    fn test_slash_starts_regex_when_allowed() {
        let tokens = tokenize("/[0-9]+/").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Regex("[0-9]+".to_string(), "".to_string())
        );
    }

    #[test]
    fn test_slash_regex_after_equals() {
        let tokens = tokenize("@x = /a/").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::At);
        assert_eq!(tokens[1].token_type, TokenType::Identifier("x".to_string()));
        assert_eq!(tokens[2].token_type, TokenType::Equals);
        assert_eq!(
            tokens[3].token_type,
            TokenType::Regex("a".to_string(), "".to_string())
        );
    }

    // --- Comment Tests ---

    #[test]
    fn test_scan_comment() {
        let tokens = tokenize("# this is a comment").unwrap();
        assert_eq!(tokens.len(), 2); // Comment + Eof
        match &tokens[0].token_type {
            TokenType::Comment(text) => assert_eq!(text, " this is a comment"),
            _ => panic!("Expected Comment token"),
        }
    }

    #[test]
    fn test_scan_comment_before_statement() {
        let tokens = tokenize("# comment\nvar").unwrap();
        assert_eq!(
            tokens[0].token_type,
            TokenType::Comment(" comment".to_string())
        );
        assert_eq!(tokens[1].token_type, TokenType::Var);
    }

    #[test]
    fn test_scan_inline_comment() {
        let tokens = tokenize("var # inline comment\nset").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Var);
        assert_eq!(
            tokens[1].token_type,
            TokenType::Comment(" inline comment".to_string())
        );
        assert_eq!(
            tokens[2].token_type,
            TokenType::Identifier("set".to_string())
        );
    }

    // --- Illegal Character Tests ---

    #[test]
    fn test_scan_at_sign() {
        let tokens = tokenize("@").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::At);
    }

    #[test]
    fn test_scan_illegal_ampersand() {
        let tokens = tokenize("&").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Illegal("&".to_string()));
    }

    // --- Complex Expression Tests ---

    #[test]
    fn test_scan_function_call() {
        assert_token_types(
            r#"var.set("x", 42)"#,
            &[
                TokenType::Var,
                TokenType::Dot,
                TokenType::Identifier("set".to_string()),
                TokenType::LeftParen,
                TokenType::String("x".to_string()),
                TokenType::Comma,
                TokenType::Number(42.0),
                TokenType::RightParen,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_try_fallback() {
        assert_token_types(
            "try { assert_eq(1, 1) } fallback { sys.echo(\"failed\") }",
            &[
                TokenType::Try,
                TokenType::LeftBrace,
                TokenType::AssertEq,
                TokenType::LeftParen,
                TokenType::Number(1.0),
                TokenType::Comma,
                TokenType::Number(1.0),
                TokenType::RightParen,
                TokenType::RightBrace,
                TokenType::Fallback,
                TokenType::LeftBrace,
                TokenType::Identifier("sys".to_string()),
                TokenType::Dot,
                TokenType::Identifier("echo".to_string()),
                TokenType::LeftParen,
                TokenType::String("failed".to_string()),
                TokenType::RightParen,
                TokenType::RightBrace,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_list_literal() {
        assert_token_types(
            "[1, 2, 3]",
            &[
                TokenType::LeftBracket,
                TokenType::Number(1.0),
                TokenType::Comma,
                TokenType::Number(2.0),
                TokenType::Comma,
                TokenType::Number(3.0),
                TokenType::RightBracket,
                TokenType::Eof,
            ],
        );
    }

    #[test]
    fn test_scan_whitespace_only() {
        let tokens = tokenize("   \t\n  ").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_scan_empty_input() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    // --- Span Tests ---

    #[test]
    fn test_token_spans_basic() {
        let tokens = tokenize("abc").unwrap();
        assert_eq!(tokens[0].span.start.line, 1);
        assert_eq!(tokens[0].span.start.column, 1);
        assert_eq!(tokens[0].span.end.column, 4);
    }

    #[test]
    fn test_token_spans_multiline() {
        let tokens = tokenize("abc\ndef").unwrap();
        assert_eq!(tokens[0].span.start.line, 1);
        assert_eq!(tokens[1].span.start.line, 2);
    }

    // --- Display Tests ---

    #[test]
    fn test_token_display() {
        let tokens = tokenize("var").unwrap();
        let display = format!("{}", tokens[0]);
        assert_eq!(display, "var @ 1:1..1:4");
    }

    #[test]
    fn test_string_display() {
        let tokens = tokenize("\"hello\"").unwrap();
        let display = format!("{}", tokens[0]);
        assert!(display.contains("\"hello\""));
    }

    // --- next_token Tests ---

    #[test]
    fn test_next_token_sequential() {
        let mut lexer = Lexer::new("var set 42");
        let t1 = lexer.next_token().unwrap();
        let t2 = lexer.next_token().unwrap();
        let t3 = lexer.next_token().unwrap();
        let t4 = lexer.next_token().unwrap();

        assert_eq!(t1.token_type, TokenType::Var);
        assert_eq!(t2.token_type, TokenType::Identifier("set".to_string()));
        assert_eq!(t3.token_type, TokenType::Number(42.0));
        assert_eq!(t4.token_type, TokenType::Eof);
    }

    #[test]
    fn test_next_token_with_whitespace() {
        let mut lexer = Lexer::new("  var  \n  set  ");
        let t1 = lexer.next_token().unwrap();
        let t2 = lexer.next_token().unwrap();
        let t3 = lexer.next_token().unwrap();

        assert_eq!(t1.token_type, TokenType::Var);
        assert_eq!(t2.token_type, TokenType::Identifier("set".to_string()));
        assert_eq!(t3.token_type, TokenType::Eof);
    }
}
