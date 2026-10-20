use crate::lexer::{Glyph, TokenKind, Tokens};
use crate::syntax::{Expression, Identifier, Span};
use element::{Failure, Key, Number, Text};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Precedence(u8);

impl Precedence {
    pub const LOWEST: Self = Self(0);
    pub const COMPARISON: Self = Self(5);
    pub const ADDITION: Self = Self(10);
    pub const MULTIPLICATION: Self = Self(20);
    pub const PREFIX: Self = Self(25);
    pub const POWER: Self = Self(30);
    pub const POSTFIX: Self = Self(40);

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Associativity {
    Left,
    Right,
    None,
}

pub struct Fixity {
    prefix: Option<Precedence>,
    infix: Option<(Precedence, Associativity)>,
    postfix: Option<Precedence>,
}

impl Fixity {
    pub fn new(
        prefix: Option<Precedence>,
        infix: Option<(Precedence, Associativity)>,
        postfix: Option<Precedence>,
    ) -> Self {
        Self {
            prefix,
            infix,
            postfix,
        }
    }

    pub fn prefix(&self) -> Option<Precedence> {
        self.prefix
    }

    pub fn infix(&self) -> Option<(Precedence, Associativity)> {
        self.infix
    }

    pub fn postfix(&self) -> Option<Precedence> {
        self.postfix
    }
}

pub trait Grammar {
    fn fixity(&self, glyph: Glyph) -> Fixity;
}

#[derive(Clone, Debug)]
pub struct Shape {
    span: Span,
    kind: ShapeKind,
}

impl Shape {
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn kind(&self) -> &ShapeKind {
        &self.kind
    }
}

#[derive(Clone, Debug)]
pub enum ShapeKind {
    Number(Number),
    Text(Text),
    Key(Key),
    Name(Identifier),
    Glyph(Glyph),
    Unary {
        operator: Box<Shape>,
        operand: Box<Shape>,
    },
    Binary {
        operator: Box<Shape>,
        left: Box<Shape>,
        right: Box<Shape>,
    },
    Postfix {
        operator: Box<Shape>,
        operand: Box<Shape>,
    },
    Multiplication {
        operator: Glyph,
        left: Box<Shape>,
        right: Box<Shape>,
    },
    Call {
        function: Box<Shape>,
        arguments: Vec<Shape>,
    },
}

pub struct Parser<'a> {
    tokens: Tokens,
    grammar: &'a dyn Grammar,
    expression: &'a Expression,
    multiplication: Option<(Glyph, Precedence)>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Tokens, grammar: &'a dyn Grammar, expression: &'a Expression) -> Self {
        Self {
            tokens,
            grammar,
            expression,
            multiplication: grammar
                .fixity(Glyph::STAR)
                .infix()
                .map(|(precedence, _)| (Glyph::STAR, precedence)),
        }
    }

    pub fn shape(&mut self) -> Result<Shape, Failure> {
        let shape = self.expression(Precedence::LOWEST)?;
        if let Some(token) = self.tokens.peek() {
            return Err(Failure::new(format!("unexpected {}", token.kind())));
        }
        Ok(shape)
    }

    fn expression(&mut self, minimum: Precedence) -> Result<Shape, Failure> {
        let mut left = self.prefix()?;
        loop {
            let kind = match self.tokens.peek() {
                Some(token) => token.kind().clone(),
                None => break,
            };
            if let TokenKind::Glyph(glyph) = kind {
                let fixity = self.grammar.fixity(glyph);
                let remainder = glyph == Glyph::PERCENT
                    && self
                        .tokens
                        .peek_after(1)
                        .is_some_and(|next| next.kind().starts_value());
                if !remainder {
                    if let Some(precedence) = fixity.postfix() {
                        if precedence >= minimum {
                            let operator = self.operator(glyph);
                            let span = left.span().start..operator.span().end;
                            left = Shape {
                                span,
                                kind: ShapeKind::Postfix {
                                    operator: Box::new(operator),
                                    operand: Box::new(left),
                                },
                            };
                            continue;
                        }
                    }
                }
                if let Some((precedence, associativity)) = fixity.infix() {
                    if precedence < minimum {
                        break;
                    }
                    let operator = self.operator(glyph);
                    let next = match associativity {
                        Associativity::Right => precedence,
                        _ => precedence.next(),
                    };
                    let right = self.expression(next)?;
                    let span = left.span().start..right.span().end;
                    left = Shape {
                        span,
                        kind: ShapeKind::Binary {
                            operator: Box::new(operator),
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                    };
                    continue;
                }
            }
            if kind.starts_value() {
                let (glyph, precedence) = match self.multiplication {
                    Some(multiplication) => multiplication,
                    None => break,
                };
                if precedence < minimum {
                    break;
                }
                let right = self.expression(precedence.next())?;
                let span = left.span().start..right.span().end;
                left = Shape {
                    span,
                    kind: ShapeKind::Multiplication {
                        operator: glyph,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                };
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Shape, Failure> {
        let token = self
            .tokens
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        let span = token.span();
        match token.kind().clone() {
            TokenKind::Number(number) => Ok(self.leaf(span, ShapeKind::Number(number))),
            TokenKind::Text(text) => Ok(self.leaf(span, ShapeKind::Text(text))),
            TokenKind::Key(key) => Ok(self.leaf(span, ShapeKind::Key(key))),
            TokenKind::Name(name) => self.name(name, span),
            TokenKind::LeftParenthesis => {
                let shape = self.expression(Precedence::LOWEST)?;
                self.expect_right_parenthesis()?;
                Ok(shape)
            }
            TokenKind::Glyph(glyph) => {
                if self.prefix_applies() {
                    if let Some(precedence) = self.grammar.fixity(glyph).prefix() {
                        let operator = self.leaf(span, ShapeKind::Glyph(glyph));
                        let operand = self.expression(precedence)?;
                        let span = operator.span().start..operand.span().end;
                        return Ok(Shape {
                            span,
                            kind: ShapeKind::Unary {
                                operator: Box::new(operator),
                                operand: Box::new(operand),
                            },
                        });
                    }
                }
                Ok(self.leaf(span, ShapeKind::Glyph(glyph)))
            }
            other => Err(Failure::new(format!("unexpected {other}"))),
        }
    }

    fn prefix_applies(&self) -> bool {
        match self.tokens.peek() {
            Some(token) => {
                token.kind().starts_value()
                    || matches!(
                        token.kind(),
                        TokenKind::Glyph(glyph) if self.grammar.fixity(*glyph).prefix().is_some()
                    )
            }
            None => false,
        }
    }

    fn name(&mut self, name: Identifier, span: Span) -> Result<Shape, Failure> {
        let function = self.leaf(span, ShapeKind::Name(name));
        if !self
            .tokens
            .peek()
            .is_some_and(|token| matches!(token.kind(), TokenKind::LeftParenthesis))
        {
            return Ok(function);
        }
        self.tokens.bump();
        let (arguments, close) = self.arguments()?;
        let span = function.span().start..self.expression.span(close).end;
        Ok(Shape {
            span,
            kind: ShapeKind::Call {
                function: Box::new(function),
                arguments,
            },
        })
    }

    fn arguments(&mut self) -> Result<(Vec<Shape>, Span), Failure> {
        if let Some(token) = self.tokens.peek() {
            if matches!(token.kind(), TokenKind::RightParenthesis) {
                let close = self.tokens.bump().expect("token was peeked").span();
                return Ok((Vec::new(), close));
            }
        }
        let mut arguments = Vec::new();
        loop {
            arguments.push(self.expression(Precedence::LOWEST)?);
            let (kind, span) = match self.tokens.peek() {
                Some(token) => (token.kind().clone(), token.span()),
                None => return Err(Failure::new("expected ',' or ')'")),
            };
            match kind {
                TokenKind::Comma => {
                    self.tokens.bump();
                }
                TokenKind::RightParenthesis => {
                    self.tokens.bump();
                    return Ok((arguments, span));
                }
                other => return Err(Failure::new(format!("expected ',' or ')', found {other}"))),
            }
        }
    }

    fn expect_right_parenthesis(&mut self) -> Result<(), Failure> {
        match self.tokens.bump() {
            Some(token) if matches!(token.kind(), TokenKind::RightParenthesis) => Ok(()),
            Some(token) => Err(Failure::new(format!(
                "expected ')', found {}",
                token.kind()
            ))),
            None => Err(Failure::new("expected ')'")),
        }
    }

    fn leaf(&self, span: Span, kind: ShapeKind) -> Shape {
        Shape {
            span: self.expression.span(span),
            kind,
        }
    }

    fn operator(&mut self, glyph: Glyph) -> Shape {
        let token = self.tokens.bump().expect("operator was peeked");
        self.leaf(token.span(), ShapeKind::Glyph(glyph))
    }
}
