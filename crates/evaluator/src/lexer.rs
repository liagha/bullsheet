use crate::syntax::{Expression, Identifier, Span};
use element::{Failure, Key, Number, Text};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Glyph(&'static str);

impl Glyph {
    pub const PLUS: Self = Self("+");
    pub const MINUS: Self = Self("-");
    pub const STAR: Self = Self("*");
    pub const SLASH: Self = Self("/");
    pub const PERCENT: Self = Self("%");
    pub const CARET: Self = Self("^");
    pub const BANG: Self = Self("!");
    pub const EQUAL: Self = Self("==");
    pub const NOT_EQUAL: Self = Self("!=");
    pub const LESS: Self = Self("<");
    pub const GREATER: Self = Self(">");
    pub const LESS_OR_EQUAL: Self = Self("<=");
    pub const GREATER_OR_EQUAL: Self = Self(">=");
}

impl std::fmt::Display for Glyph {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl TryFrom<char> for Glyph {
    type Error = Failure;

    fn try_from(character: char) -> Result<Self, Self::Error> {
        match character {
            '+' => Ok(Self::PLUS),
            '-' => Ok(Self::MINUS),
            '*' => Ok(Self::STAR),
            '/' => Ok(Self::SLASH),
            '%' => Ok(Self::PERCENT),
            '^' => Ok(Self::CARET),
            '!' => Ok(Self::BANG),
            '<' => Ok(Self::LESS),
            '>' => Ok(Self::GREATER),
            _ => Err(Failure::new(format!("unexpected character {character}"))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TokenKind {
    Number(Number),
    Name(Identifier),
    Key(Key),
    Glyph(Glyph),
    LeftParenthesis,
    RightParenthesis,
    Comma,
}

impl TokenKind {
    pub fn starts_value(&self) -> bool {
        matches!(
            self,
            Self::Number(_) | Self::Name(_) | Self::Key(_) | Self::LeftParenthesis
        )
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(formatter, "{}", number.value()),
            Self::Name(name) => write!(formatter, "{name}"),
            Self::Key(key) => write!(formatter, "{key}"),
            Self::Glyph(glyph) => write!(formatter, "{glyph}"),
            Self::LeftParenthesis => formatter.write_str("("),
            Self::RightParenthesis => formatter.write_str(")"),
            Self::Comma => formatter.write_str(","),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    kind: TokenKind,
    span: Span,
}

impl Token {
    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }

    pub fn span(&self) -> Span {
        self.span.clone()
    }
}

#[derive(Debug)]
pub struct Tokens {
    items: Vec<Token>,
    position: usize,
}

impl Tokens {
    fn new(items: Vec<Token>) -> Self {
        Self { items, position: 0 }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.items.get(self.position)
    }

    pub fn peek_after(&self, offset: usize) -> Option<&Token> {
        self.items.get(self.position + offset)
    }

    pub fn bump(&mut self) -> Option<Token> {
        let token = self.items.get(self.position).cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }
}

pub struct Lexer {
    characters: Vec<char>,
    position: usize,
    depth: usize,
}

impl Lexer {
    pub fn new(expression: &Expression) -> Self {
        Self {
            characters: expression.as_ref().chars().collect(),
            position: 0,
            depth: 0,
        }
    }

    pub fn tokens(&mut self) -> Result<Tokens, Failure> {
        let mut items = Vec::new();
        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                self.bump();
                continue;
            }
            let start = self.position;
            let kind = self.token()?;
            items.push(Token {
                kind,
                span: start..self.position,
            });
        }
        Ok(Tokens::new(items))
    }

    fn token(&mut self) -> Result<TokenKind, Failure> {
        let character = self
            .peek()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        match character {
            '(' => {
                self.bump();
                self.depth += 1;
                Ok(TokenKind::LeftParenthesis)
            }
            ')' => {
                self.bump();
                self.depth = self.depth.saturating_sub(1);
                Ok(TokenKind::RightParenthesis)
            }
            ',' => {
                self.bump();
                Ok(TokenKind::Comma)
            }
            _ if character.is_ascii_digit() => self.number(),
            _ if character == '.'
                && self.peek_after(1).is_some_and(|next| next.is_ascii_digit()) =>
            {
                self.number()
            }
            _ if character.is_ascii_alphabetic() || character == '_' => Ok(self.identifier()),
            _ => self.symbol(),
        }
    }

    fn number(&mut self) -> Result<TokenKind, Failure> {
        if self.peek() == Some('0') {
            match self.peek_after(1) {
                Some('x' | 'X') => {
                    self.bump();
                    self.bump();
                    return self.radix(16);
                }
                Some('b' | 'B') => {
                    self.bump();
                    self.bump();
                    return self.radix(2);
                }
                _ => {}
            }
        }
        let mut text = String::new();
        self.digits(10, &mut text);
        while self.depth == 0 && self.grouped() {
            self.bump();
            self.digits(10, &mut text);
        }
        if self.peek() == Some('.') {
            if let Some(character) = self.bump() {
                text.push(character);
            }
            self.digits(10, &mut text);
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let mut lookahead = 1;
            let signed = matches!(self.peek_after(1), Some('+' | '-'));
            if signed {
                lookahead = 2;
            }
            if self
                .peek_after(lookahead)
                .is_some_and(|character| character.is_ascii_digit())
            {
                if let Some(character) = self.bump() {
                    text.push(character);
                }
                if signed {
                    if let Some(character) = self.bump() {
                        text.push(character);
                    }
                }
                self.digits(10, &mut text);
            }
        }
        let number = text
            .parse::<f64>()
            .map_err(|_| Failure::new("invalid number"))?;
        Ok(TokenKind::Number(Number::new(number)))
    }

    fn grouped(&self) -> bool {
        self.peek() == Some(',')
            && (1..=3).all(|offset| {
                self.peek_after(offset)
                    .is_some_and(|character| character.is_ascii_digit())
            })
            && !self
                .peek_after(4)
                .is_some_and(|character| character.is_ascii_digit())
    }

    fn radix(&mut self, radix: u32) -> Result<TokenKind, Failure> {
        let mut text = String::new();
        self.digits(radix, &mut text);
        if text.is_empty() {
            return Err(Failure::new("missing digits"));
        }
        let number =
            u128::from_str_radix(&text, radix).map_err(|_| Failure::new("invalid number"))?;
        Ok(TokenKind::Number(Number::new(number as f64)))
    }

    fn digits(&mut self, radix: u32, text: &mut String) {
        while let Some(character) = self.peek() {
            if character == '_' {
                self.bump();
                continue;
            }
            if character.to_digit(radix).is_none() {
                break;
            }
            if let Some(character) = self.bump() {
                text.push(character);
            }
        }
    }

    fn identifier(&mut self) -> TokenKind {
        let mut text = String::new();
        while let Some(character) = self.peek() {
            if character.is_ascii_alphanumeric() || character == '_' {
                if let Some(character) = self.bump() {
                    text.push(character);
                }
            } else {
                break;
            }
        }
        if let Some(key) = Self::key(&text) {
            return TokenKind::Key(key);
        }
        TokenKind::Name(Identifier::new(&text))
    }

    fn key(text: &str) -> Option<Key> {
        let modifiers = match text {
            "Ctrl" => element::Modifiers::CONTROL,
            "Alt" => element::Modifiers::ALT,
            "Shift" => element::Modifiers::SHIFT,
            "Super" => element::Modifiers::SUPER,
            _ if Self::key_name(text) => {
                return Some(Key::new(element::Modifiers::NONE, Text::new(text)));
            }
            _ => return None,
        };
        Some(Key::modifier(modifiers))
    }

    fn key_name(text: &str) -> bool {
        let mut characters = text.chars();
        if characters
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
            && characters.next().is_none()
        {
            return true;
        }
        if text
            .strip_prefix('F')
            .and_then(|rest| rest.parse::<u8>().ok())
            .is_some_and(|number| (1..=24).contains(&number))
        {
            return true;
        }
        matches!(
            text,
            "Space"
                | "Enter"
                | "Escape"
                | "Tab"
                | "Backspace"
                | "Delete"
                | "Insert"
                | "Home"
                | "End"
                | "PageUp"
                | "PageDown"
                | "Left"
                | "Right"
                | "Up"
                | "Down"
        )
    }

    fn symbol(&mut self) -> Result<TokenKind, Failure> {
        let character = self
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        let pair = match (character, self.peek()) {
            ('*', Some('*')) => Some(Glyph::CARET),
            ('=', Some('=')) => Some(Glyph::EQUAL),
            ('!', Some('=')) => Some(Glyph::NOT_EQUAL),
            ('<', Some('=')) => Some(Glyph::LESS_OR_EQUAL),
            ('>', Some('=')) => Some(Glyph::GREATER_OR_EQUAL),
            _ => None,
        };
        if let Some(glyph) = pair {
            self.bump();
            return Ok(TokenKind::Glyph(glyph));
        }
        Ok(TokenKind::Glyph(Glyph::try_from(character)?))
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn peek_after(&self, offset: usize) -> Option<char> {
        self.characters.get(self.position + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek();
        if character.is_some() {
            self.position += 1;
        }
        character
    }
}
