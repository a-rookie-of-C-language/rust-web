use crate::ast::expression_node::{BinaryOp, Expr, UnaryOp};

/// Recursive-descent parser for SpEL expressions.
///
/// **Grammar (simplified BNF):**
/// ```text
/// expr         := ternary
/// ternary      := or_expr ('?' expr ':' expr)?
/// or_expr      := and_expr ('||' and_expr)*
/// and_expr     := not_expr ('&&' not_expr)*
/// not_expr     := '!' not_expr | comparison
/// comparison   := addition (('==' | '!=' | '<' | '<=' | '>' | '>=') addition)?
/// addition     := multiplication (('+' | '-') multiplication)*
/// multiplication := unary (('*' | '/' | '%') unary)*
/// unary        := '-' unary | primary
/// primary      := literal | placeholder | '(' expr ')' | ident_or_method
/// literal      := INTEGER | FLOAT | STRING | BOOL | NULL
/// placeholder  := '${' key (':' default)? '}'
/// ident_or_method := IDENT ('.' IDENT '(' args ')')*
/// args         := (expr (',' expr)*)?
/// ```
pub struct SpelParser {
    chars: Vec<char>,
    pos: usize,
}

impl SpelParser {
    pub fn new(input: &str) -> Self {
        SpelParser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    // ── public entry ──────────────────────────────────────────────────────

    pub fn parse(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        let expr = self.parse_ternary()?;
        self.skip_ws();
        if self.pos < self.chars.len() {
            return Err(format!(
                "unexpected character '{}' at position {}",
                self.chars[self.pos], self.pos
            ));
        }
        Ok(expr)
    }

    // ── grammar rules ─────────────────────────────────────────────────────

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_or()?;
        self.skip_ws();
        if self.peek() == Some('?') {
            self.advance();
            self.skip_ws();
            let then_e = self.parse_ternary()?;
            self.skip_ws();
            self.expect(':')?;
            self.skip_ws();
            let else_e = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then_e: Box::new(then_e),
                else_e: Box::new(else_e),
            });
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.peek2() == Some(['|', '|']) {
                self.advance();
                self.advance();
                self.skip_ws();
                let right = self.parse_and()?;
                left = Expr::Binary {
                    op: BinaryOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;
        loop {
            self.skip_ws();
            if self.peek2() == Some(['&', '&']) {
                self.advance();
                self.advance();
                self.skip_ws();
                let right = self.parse_not()?;
                left = Expr::Binary {
                    op: BinaryOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.peek() == Some('!') && self.peek_at(1) != Some('=') {
            self.advance();
            let expr = self.parse_not()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_addition()?;
        self.skip_ws();
        let op = match (self.peek(), self.peek_at(1)) {
            (Some('='), Some('=')) => {
                self.advance();
                self.advance();
                BinaryOp::Eq
            }
            (Some('!'), Some('=')) => {
                self.advance();
                self.advance();
                BinaryOp::Ne
            }
            (Some('<'), Some('=')) => {
                self.advance();
                self.advance();
                BinaryOp::Le
            }
            (Some('>'), Some('=')) => {
                self.advance();
                self.advance();
                BinaryOp::Ge
            }
            (Some('<'), _) => {
                self.advance();
                BinaryOp::Lt
            }
            (Some('>'), _) => {
                self.advance();
                BinaryOp::Gt
            }
            _ => return Ok(left),
        };
        self.skip_ws();
        let right = self.parse_addition()?;
        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.advance();
                    self.skip_ws();
                    let r = self.parse_multiplication()?;
                    left = Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(left),
                        right: Box::new(r),
                    };
                }
                Some('-') => {
                    self.advance();
                    self.skip_ws();
                    let r = self.parse_multiplication()?;
                    left = Expr::Binary {
                        op: BinaryOp::Sub,
                        left: Box::new(left),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.advance();
                    self.skip_ws();
                    let r = self.parse_unary()?;
                    left = Expr::Binary {
                        op: BinaryOp::Mul,
                        left: Box::new(left),
                        right: Box::new(r),
                    };
                }
                Some('/') => {
                    self.advance();
                    self.skip_ws();
                    let r = self.parse_unary()?;
                    left = Expr::Binary {
                        op: BinaryOp::Div,
                        left: Box::new(left),
                        right: Box::new(r),
                    };
                }
                Some('%') => {
                    self.advance();
                    self.skip_ws();
                    let r = self.parse_unary()?;
                    left = Expr::Binary {
                        op: BinaryOp::Rem,
                        left: Box::new(left),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.peek() == Some('-') {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_postfix()
    }

    /// Parse a primary and then any `.method(args)` suffixes.
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut node = self.parse_primary()?;
        loop {
            self.skip_ws();
            if self.peek() == Some('.') {
                self.advance();
                let method = self.parse_ident()?;
                self.skip_ws();
                self.expect('(')?;
                let args = self.parse_args()?;
                self.expect(')')?;
                node = Expr::MethodCall {
                    target: Box::new(node),
                    method,
                    args,
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.advance();
                let e = self.parse_ternary()?;
                self.skip_ws();
                self.expect(')')?;
                Ok(e)
            }
            Some('$') if self.peek_at(1) == Some('{') => self.parse_placeholder(),
            Some('"') | Some('\'') => self.parse_string_lit(),
            Some(c) if c.is_ascii_digit() => self.parse_number(),
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.parse_ident()?;
                match ident.as_str() {
                    "true" => Ok(Expr::BoolLit(true)),
                    "false" => Ok(Expr::BoolLit(false)),
                    "null" => Ok(Expr::Null),
                    _ => Ok(Expr::Identifier(ident)),
                }
            }
            other => Err(format!("unexpected token {:?} at pos {}", other, self.pos)),
        }
    }

    fn parse_placeholder(&mut self) -> Result<Expr, String> {
        // consume '${'
        self.advance();
        self.advance();
        let mut key = String::new();
        while let Some(c) = self.peek() {
            if c == ':' || c == '}' {
                break;
            }
            key.push(c);
            self.advance();
        }
        let default = if self.peek() == Some(':') {
            self.advance();
            let mut d = String::new();
            while let Some(c) = self.peek() {
                if c == '}' {
                    break;
                }
                d.push(c);
                self.advance();
            }
            Some(d)
        } else {
            None
        };
        self.expect('}')?;
        Ok(Expr::PropertyPlaceholder {
            key: key.trim().to_string(),
            default,
        })
    }

    fn parse_string_lit(&mut self) -> Result<Expr, String> {
        let quote = self.peek().unwrap();
        self.advance();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err("unterminated string literal".into()),
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n') => {
                            s.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            s.push('\t');
                            self.advance();
                        }
                        Some('\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some(c) if c == quote => {
                            s.push(c);
                            self.advance();
                        }
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                            self.advance();
                        }
                        None => return Err("unterminated escape".into()),
                    }
                }
                Some(c) if c == quote => {
                    self.advance();
                    break;
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        Ok(Expr::StringLit(s))
    }

    fn parse_number(&mut self) -> Result<Expr, String> {
        let mut s = String::new();
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                is_float = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if is_float {
            s.parse::<f64>()
                .map(Expr::FloatLit)
                .map_err(|e| e.to_string())
        } else {
            s.parse::<i64>()
                .map(Expr::IntLit)
                .map_err(|e| e.to_string())
        }
    }

    fn parse_ident(&mut self) -> Result<String, String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if s.is_empty() {
            Err(format!("expected identifier at pos {}", self.pos))
        } else {
            Ok(s)
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        self.skip_ws();
        if self.peek() == Some(')') {
            return Ok(vec![]);
        }
        let mut args = vec![self.parse_ternary()?];
        loop {
            self.skip_ws();
            if self.peek() == Some(',') {
                self.advance();
                self.skip_ws();
                args.push(self.parse_ternary()?);
            } else {
                break;
            }
        }
        Ok(args)
    }

    // ── character-level helpers ───────────────────────────────────────────

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn peek2(&self) -> Option<[char; 2]> {
        Some([
            self.chars.get(self.pos).copied()?,
            self.chars.get(self.pos + 1).copied()?,
        ])
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_ws(&mut self) {
        while matches!(
            self.peek(),
            Some(' ') | Some('\t') | Some('\n') | Some('\r')
        ) {
            self.advance();
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        match self.peek() {
            Some(got) if got == c => {
                self.advance();
                Ok(())
            }
            got => Err(format!(
                "expected '{}', got {:?} at pos {}",
                c, got, self.pos
            )),
        }
    }
}
