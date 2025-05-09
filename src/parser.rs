use crate::error::{Result, eof_parse_error, parse_error};
use crate::lexer::{Token, TokenType};
use crate::program::{BinaryOp, Declaration, Expr, Literal, Program, Statement, UnaryOp, binop};

pub struct Parser {
    pub tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Given an iterator of tokens, construct a parser for those
    /// tokens.
    pub fn from_tokens(tokens: impl IntoIterator<Item = Token>) -> Self {
        let p = Parser {
            tokens: tokens.into_iter().collect(),
            current: 0,
        };
        for token in &p.tokens {
            println!("{:?}", token);
        }
        p
    }

    /// The main entry point to try to parse the provided sequence of
    /// tokens.  Either succeeds and gives you a program, or else
    /// returns a parse error.
    pub fn parse(&mut self) -> Result<Program> {
        let mut decls = Vec::new();
        loop {
            if let Some(TokenType::Eof) = self.peek_token_type() {
                return Ok(decls);
            } else {
                let stmt = self.declaration()?;
                decls.push(stmt);
            }
        }
    }

    /// Peek at just the type of the current token, useful for simpler
    /// pattern matching.
    fn peek_token_type(&self) -> Option<&TokenType> {
        self.tokens.get(self.current).map(|t| &t.token_type)
    }

    /// Peek at the token after the current one, which makes it easier
    /// to distinguish between, e.g., = and ==
    fn peek_next_token_type(&self) -> Option<&TokenType> {
        self.tokens.get(self.current + 1).map(|t| &t.token_type)
    }

    /// Get at the current token, including all metadata
    fn current_token(&mut self) -> Result<&Token> {
        self.tokens
            .get(self.current)
            .ok_or(eof_parse_error::<&Token>())
    }

    /// Move forward in the sequence of tokens
    fn consume(&mut self) {
        self.current += 1;
    }

    /// Move backward in the sequence of tokens
    fn unconsume(&mut self) {
        self.current -= 1;
    }

    /// Try to consume a token of a particular type.  For instance,
    /// make sure we find a semicolon after parsing a statement
    fn consume_type(&mut self, ttype: TokenType) -> Option<&Token> {
        match self.peek_token_type() {
            Some(t) if *t == ttype => {
                self.consume();
                self.current_token().ok()
            }
            _ => None,
        }
    }

    fn declaration(&mut self) -> Result<Declaration> {
        match self.peek_token_type() {
            Some(TokenType::Var) => self.var_declaration(),
            _ => Ok(Declaration::Statement(self.statement()?)),
        }
    }

    fn var_declaration(&mut self) -> Result<Declaration> {
        self.consume_type(TokenType::Var);
        let ident = self.current_token().unwrap().lexeme.clone();

        // TODO: Because the data is owned by the enum variant, it's
        // awkard to match it by type.
        self.consume();

        match self.peek_token_type() {
            Some(TokenType::Equal) => {
                self.consume();
                let rhs = self.expr()?;
                self.consume_type(TokenType::SemiColon);
                Ok(Declaration::Variable {
                    identifier: ident,
                    value: Some(rhs),
                })
            }
            Some(TokenType::SemiColon) => {
                self.consume();
                Ok(Declaration::Variable {
                    identifier: ident,
                    value: None,
                })
            }
            _ => Err(parse_error::<Declaration>("Malformed variable declaration")),
        }
    }

    fn statement(&mut self) -> Result<Statement> {
        match self.peek_token_type() {
            Some(TokenType::Print) => {
                self.consume();
                let rhs = self.expr()?;
                self.consume_type(TokenType::SemiColon);
                Ok(Statement::Print(rhs))
            }
            Some(TokenType::Eof) | None => Err(eof_parse_error::<Statement>()),
            Some(_) => {
                let rhs = self.expr()?;
                match self.consume_type(TokenType::SemiColon) {
                    Some(_) => Ok(Statement::Expr(rhs)),
                    _ => Err(parse_error::<Statement>("Missing semicolon")),
                }
            }
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr> {
        let lhs = self.comparison()?;
        loop {
            if let Some(op) = self.to_equality_op() {
                self.consume();
                let rhs = self.comparison()?;
                return Ok(binop(lhs, op, rhs));
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn to_equality_op(&mut self) -> Option<BinaryOp> {
        let token = self.current_token().ok()?;
        match &token.token_type {
            TokenType::EqualEqual => Some(BinaryOp::Equal),
            TokenType::BangEqual => Some(BinaryOp::NotEqual),
            _ => None,
        }
    }

    fn comparison(&mut self) -> Result<Expr> {
        let lhs = self.term()?;
        loop {
            if let Some(op) = self.to_comparison_op() {
                self.consume();
                let rhs = self.term()?;
                return Ok(binop(lhs, op, rhs));
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn to_comparison_op(&mut self) -> Option<BinaryOp> {
        let token = self.current_token().ok()?;
        match &token.token_type {
            TokenType::Less => Some(BinaryOp::Less),
            TokenType::LessEqual => Some(BinaryOp::LessEqual),
            TokenType::Greater => Some(BinaryOp::Greater),
            TokenType::GreaterEqual => Some(BinaryOp::GreaterEqual),
            _ => None,
        }
    }

    fn term(&mut self) -> Result<Expr> {
        let lhs = self.factor()?;
        loop {
            if let Some(op) = self.to_term_op() {
                self.consume();
                let rhs = self.factor()?;
                return Ok(binop(lhs, op, rhs));
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn to_term_op(&mut self) -> Option<BinaryOp> {
        let token = self.current_token().ok()?;
        match &token.token_type {
            TokenType::Minus => Some(BinaryOp::Minus),
            TokenType::Plus => Some(BinaryOp::Plus),
            _ => None,
        }
    }

    fn factor(&mut self) -> Result<Expr> {
        let lhs = self.unary()?;
        loop {
            if let Some(op) = self.to_factor_op() {
                self.consume();
                let rhs = self.unary()?;
                return Ok(binop(lhs, op, rhs));
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn to_factor_op(&mut self) -> Option<BinaryOp> {
        let token = self.current_token().ok()?;
        match &token.token_type {
            TokenType::Slash => Some(BinaryOp::Div),
            TokenType::Star => Some(BinaryOp::Times),
            _ => None,
        }
    }

    fn unary(&mut self) -> Result<Expr> {
        if let Some(op) = self.to_unary_op() {
            self.consume();
            let rhs = self.unary()?;
            Ok(Expr::Unary(op, Box::new(rhs)))
        } else {
            self.primary()
        }
    }

    fn to_unary_op(&mut self) -> Option<UnaryOp> {
        let token = self.current_token().ok()?;
        match &token.token_type {
            TokenType::Bang => Some(UnaryOp::Not),
            TokenType::Minus => Some(UnaryOp::Negate),
            _ => None,
        }
    }

    fn primary(&mut self) -> Result<Expr> {
        let token = self.current_token().unwrap();
        match &token.token_type {
            TokenType::False => {
                self.consume();
                Ok(Expr::Literal(Literal::False))
            }
            TokenType::True => {
                self.consume();
                Ok(Expr::Literal(Literal::True))
            }
            TokenType::Nil => {
                self.consume();
                Ok(Expr::Literal(Literal::Nil))
            }
            TokenType::Number { literal } => {
                let expr = Expr::Literal(Literal::Number(*literal));
                self.consume();
                Ok(expr)
            }
            TokenType::String { literal } => {
                let expr = Expr::Literal(Literal::String(literal.to_string()));
                self.consume();
                Ok(expr)
            }
            TokenType::LeftParen => {
                self.consume();
                let expr = self.expr()?;
                match self.consume_type(TokenType::RightParen) {
                    Some(_) => Ok(Expr::Grouping(Box::new(expr))),
                    _ => Err(parse_error::<Expr>("Failed to find expected closing paren")),
                }
            }
            TokenType::Identifier { name } => {
                let id = name.clone();
                self.consume();
                Ok(Expr::Literal(Literal::Identifier(id)))
            }
            _ => Err(parse_error::<Expr>(&format!(
                "Line {:?}: Unable to parse expression {:?} as token of type {:?}",
                token.line, token.lexeme, token.token_type,
            ))),
        }
    }
}
