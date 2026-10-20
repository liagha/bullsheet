use element::{Blank, Callable, Failure, Function, Key, Number, Text, Truth, Value};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Identifier(String);

impl Identifier {
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_ascii_lowercase())
    }

    pub(crate) fn is_answer(&self) -> bool {
        self.0 == "ans"
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<&str> for Identifier {
    type Error = Failure;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let trimmed = text.trim();
        let mut characters = trimmed.chars();
        match characters.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
            _ => return Err(Failure::new("invalid name")),
        }
        if characters.any(|character| !(character.is_ascii_alphanumeric() || character == '_')) {
            return Err(Failure::new("invalid name"));
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }
}


#[derive(Debug)]
pub struct Source(String);

impl Source {
    pub fn new(text: String) -> Self {
        Self(text)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.0.split('\n').count()
    }

    pub(crate) fn lines(&self) -> impl Iterator<Item = Line<'_>> {
        self.0.split('\n').map(Line::new)
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Source {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<String> for Source {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<String> for Source {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for Source {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}

pub(crate) struct Line<'a> {
    text: &'a str,
}

impl<'a> Line<'a> {
    fn new(text: &'a str) -> Self {
        Self { text }
    }

    fn statement(&self) -> Result<Statement, Failure> {
        let code = self.without_comment().trim();
        if code.is_empty() {
            return Ok(Statement::Empty);
        }
        let offset = self.text.chars().count() - self.text.trim_start().chars().count();
        if let Some((left, right)) = Self::assignment(code) {
            let expression = Expression::new(right, offset + left.chars().count() + 1);
            if expression.is_empty() {
                return Err(Failure::new("missing expression"));
            }
            if let Some((name, span, parameters)) = Self::definition(left, offset)? {
                return Ok(Statement::Definition {
                    name,
                    span,
                    parameters,
                    expression,
                });
            }
            let name = Identifier::try_from(left)?;
            let start = offset + left.chars().count() - left.trim_start().chars().count();
            let span = start..start + left.trim().chars().count();
            return Ok(Statement::Assignment {
                name,
                span,
                expression,
            });
        }
        let expression = Expression::new(code, offset);
        if expression.is_empty() {
            return Ok(Statement::Empty);
        }
        Ok(Statement::Expression(expression))
    }

    fn assignment(code: &str) -> Option<(&str, &str)> {
        let characters: Vec<char> = code.chars().collect();
        let mut depth = 0usize;
        for (index, character) in characters.iter().enumerate() {
            match character {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                '=' if depth == 0 => {
                    let previous = index.checked_sub(1).map(|earlier| characters[earlier]);
                    let next = characters.get(index + 1);
                    if matches!(previous, Some('=' | '<' | '>' | '!')) || next == Some(&'=') {
                        continue;
                    }
                    let byte = code.char_indices().nth(index)?.0;
                    return Some((&code[..byte], &code[byte + 1..]));
                }
                _ => {}
            }
        }
        None
    }

    fn definition(
        left: &str,
        offset: usize,
    ) -> Result<
        Option<(
            Identifier,
            std::ops::Range<usize>,
            Vec<(Identifier, std::ops::Range<usize>)>,
        )>,
        Failure,
    > {
        let start = offset + left.chars().count() - left.trim_start().chars().count();
        let left = left.trim();
        if !left.ends_with(')') {
            return Ok(None);
        }
        let open = match left.find('(') {
            Some(open) => open,
            None => return Ok(None),
        };
        let name = Identifier::try_from(left[..open].trim())?;
        let span = start..start + left[..open].trim().chars().count();
        let mut parameters = Vec::new();
        let mut cursor = start + left[..open].chars().count() + 1;
        for piece in left[open + 1..left.len() - 1].split(',') {
            let parameter = piece.trim();
            if !parameter.is_empty() {
                let begin = cursor + piece.chars().count() - piece.trim_start().chars().count();
                parameters.push((
                    Identifier::try_from(parameter)?,
                    begin..begin + parameter.chars().count(),
                ));
            }
            cursor += piece.chars().count() + 1;
        }
        Ok(Some((name, span, parameters)))
    }

    fn without_comment(&self) -> &'a str {
        self.text
            .split_once('#')
            .map_or(self.text, |(code, _)| code)
    }
}

#[derive(Debug)]
struct Expression {
    text: String,
    offset: usize,
    sources: Vec<usize>,
}

impl Expression {
    fn new(text: &str, offset: usize) -> Self {
        let mut normalized = String::with_capacity(text.len());
        let mut sources = Vec::with_capacity(text.len());
        for (index, character) in text.chars().enumerate() {
            match character {
                '\u{06F0}'..='\u{06F9}' => {
                    let digit = char::from_u32('0' as u32 + (character as u32 - '\u{06F0}' as u32))
                        .unwrap_or('0');
                    normalized.push(digit);
                    sources.push(index);
                }
                '\u{0660}'..='\u{0669}' => {
                    let digit = char::from_u32('0' as u32 + (character as u32 - '\u{0660}' as u32))
                        .unwrap_or('0');
                    normalized.push(digit);
                    sources.push(index);
                }
                '×' | '·' => {
                    normalized.push('*');
                    sources.push(index);
                }
                '÷' => {
                    normalized.push('/');
                    sources.push(index);
                }
                '−' | '–' | '—' => {
                    normalized.push('-');
                    sources.push(index);
                }
                '٬' | '،' => {
                    normalized.push(',');
                    sources.push(index);
                }
                '٫' => {
                    normalized.push('.');
                    sources.push(index);
                }
                '٪' => {
                    normalized.push('%');
                    sources.push(index);
                }
                'π' => Self::expanded(&mut normalized, &mut sources, "pi", index),
                'τ' => Self::expanded(&mut normalized, &mut sources, "tau", index),
                '∞' => Self::expanded(&mut normalized, &mut sources, "inf", index),
                '\u{200E}' | '\u{200F}' => {}
                _ => {
                    normalized.push(character);
                    sources.push(index);
                }
            }
        }
        let trimmed = normalized.trim();
        let leading = normalized.chars().count() - normalized.trim_start().chars().count();
        let trailing = normalized.chars().count() - normalized.trim_end().chars().count();
        sources.drain(..leading);
        sources.truncate(sources.len().saturating_sub(trailing));
        Self {
            text: trimmed.to_string(),
            offset,
            sources,
        }
    }

    fn expanded(normalized: &mut String, sources: &mut Vec<usize>, text: &str, index: usize) {
        for _ in text.chars() {
            sources.push(index);
        }
        normalized.push_str(text);
    }

    fn span(&self, span: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let start = self.sources.get(span.start).copied().unwrap_or(0);
        let end = span
            .end
            .checked_sub(1)
            .and_then(|last| self.sources.get(last))
            .map_or(start, |last| last + 1);
        (self.offset + start)..(self.offset + end)
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl AsRef<str> for Expression {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

#[derive(Debug)]
enum Statement {
    Empty,
    Expression(Expression),
    Assignment {
        name: Identifier,
        span: std::ops::Range<usize>,
        expression: Expression,
    },
    Definition {
        name: Identifier,
        span: std::ops::Range<usize>,
        parameters: Vec<(Identifier, std::ops::Range<usize>)>,
        expression: Expression,
    },
}

pub struct Part {
    span: std::ops::Range<usize>,
    value: Value,
}

impl Part {
    pub fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

pub struct Reading {
    value: Value,
    parts: Vec<Part>,
}

impl Reading {
    fn new(value: Value, parts: Vec<Part>) -> Self {
        Self { value, parts }
    }

    fn without_parts(value: Value) -> Self {
        Self::new(value, Vec::new())
    }

    fn replace(&mut self, value: Value) {
        self.value = value;
    }

    fn prefix(&mut self, mut parts: Vec<Part>) {
        parts.append(&mut self.parts);
        self.parts = parts;
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn parts(&self) -> &[Part] {
        &self.parts
    }
}

pub struct Name {
    text: Text,
    value: Value,
}

impl Name {
    pub fn text(&self) -> &Text {
        &self.text
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

pub struct Readings {
    lines: Vec<Reading>,
    names: Vec<Name>,
}

impl Readings {
    fn new(lines: Vec<Reading>, names: Vec<Name>) -> Self {
        Self { lines, names }
    }

    pub fn names(&self) -> &[Name] {
        &self.names
    }
}

impl std::ops::Deref for Readings {
    type Target = [Reading];

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

impl IntoIterator for Readings {
    type Item = Reading;
    type IntoIter = std::vec::IntoIter<Reading>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl<'a> IntoIterator for &'a Readings {
    type Item = &'a Reading;
    type IntoIter = std::slice::Iter<'a, Reading>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.iter()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Symbol(&'static str);

impl Symbol {
    const PLUS: Self = Self("+");
    const MINUS: Self = Self("-");
    const STAR: Self = Self("*");
    const SLASH: Self = Self("/");
    const PERCENT: Self = Self("%");
    const CARET: Self = Self("^");
    const BANG: Self = Self("!");
    const EQUAL: Self = Self("==");
    const NOT_EQUAL: Self = Self("!=");
    const LESS: Self = Self("<");
    const GREATER: Self = Self(">");
    const LESS_OR_EQUAL: Self = Self("<=");
    const GREATER_OR_EQUAL: Self = Self(">=");
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl TryFrom<char> for Symbol {
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
enum Token {
    Number(Number),
    Name(Identifier),
    Key(Key),
    Symbol(Symbol),
    LeftParenthesis,
    RightParenthesis,
    Comma,
}

impl Token {
    fn starts_value(&self) -> bool {
        matches!(
            self,
            Self::Number(_) | Self::Name(_) | Self::Key(_) | Self::LeftParenthesis
        )
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(formatter, "{}", number.value()),
            Self::Name(name) => write!(formatter, "{name}"),
            Self::Key(key) => write!(formatter, "{key}"),
            Self::Symbol(symbol) => write!(formatter, "{symbol}"),
            Self::LeftParenthesis => formatter.write_str("("),
            Self::RightParenthesis => formatter.write_str(")"),
            Self::Comma => formatter.write_str(","),
        }
    }
}

#[derive(Debug)]
struct Tokens {
    items: Vec<(Token, std::ops::Range<usize>)>,
    position: usize,
}

impl Tokens {
    fn new(items: Vec<(Token, std::ops::Range<usize>)>) -> Self {
        Self { items, position: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.items.get(self.position).map(|(token, _)| token)
    }

    fn peek_after(&self, offset: usize) -> Option<&Token> {
        self.items
            .get(self.position + offset)
            .map(|(token, _)| token)
    }

    fn span(&self, index: usize) -> Option<std::ops::Range<usize>> {
        self.items.get(index).map(|(_, span)| span.clone())
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.peek().cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn is_empty(&self) -> bool {
        self.peek().is_none()
    }

    fn lazy_arguments(
        &mut self,
    ) -> Result<Vec<Vec<(Token, std::ops::Range<usize>)>>, Failure> {
        let mut arguments = Vec::new();
        let mut current = Vec::new();
        let mut depth = 0usize;
        loop {
            let item = match self.items.get(self.position).cloned() {
                Some(item) => item,
                None => return Err(Failure::new("expected ')'")),
            };
            self.position += 1;
            match &item.0 {
                Token::LeftParenthesis => {
                    depth += 1;
                    current.push(item);
                }
                Token::RightParenthesis if depth == 0 => {
                    if !current.is_empty() || !arguments.is_empty() {
                        arguments.push(current);
                    }
                    return Ok(arguments);
                }
                Token::RightParenthesis => {
                    depth = depth.saturating_sub(1);
                    current.push(item);
                }
                Token::Comma if depth == 0 => arguments.push(std::mem::take(&mut current)),
                _ => current.push(item),
            }
        }
    }
}

struct Lexer {
    characters: Vec<char>,
    position: usize,
    depth: usize,
}

impl Lexer {
    fn new(expression: &Expression) -> Self {
        Self {
            characters: expression.as_ref().chars().collect(),
            position: 0,
            depth: 0,
        }
    }

    fn tokens(&mut self) -> Result<Tokens, Failure> {
        let mut items = Vec::new();
        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                self.bump();
                continue;
            }
            let start = self.position;
            let token = self.token()?;
            items.push((token, start..self.position));
        }
        Ok(Tokens::new(items))
    }

    fn token(&mut self) -> Result<Token, Failure> {
        let character = self
            .peek()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        match character {
            '(' => {
                self.bump();
                self.depth += 1;
                Ok(Token::LeftParenthesis)
            }
            ')' => {
                self.bump();
                self.depth = self.depth.saturating_sub(1);
                Ok(Token::RightParenthesis)
            }
            ',' => {
                self.bump();
                Ok(Token::Comma)
            }
            _ if character.is_ascii_digit() => self.number(),
            _ if character == '.'
                && self
                    .peek_after(1)
                    .is_some_and(|next| next.is_ascii_digit()) =>
            {
                self.number()
            }
            _ if character.is_ascii_alphabetic() || character == '_' => Ok(self.identifier()),
            _ => self.symbol(),
        }
    }

    fn number(&mut self) -> Result<Token, Failure> {
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
        Ok(Token::Number(Number::new(number)))
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

    fn radix(&mut self, radix: u32) -> Result<Token, Failure> {
        let mut text = String::new();
        self.digits(radix, &mut text);
        if text.is_empty() {
            return Err(Failure::new("missing digits"));
        }
        let number = u128::from_str_radix(&text, radix)
            .map_err(|_| Failure::new("invalid number"))?;
        Ok(Token::Number(Number::new(number as f64)))
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

    fn identifier(&mut self) -> Token {
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
            return Token::Key(key);
        }
        Token::Name(Identifier::new(&text))
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

    fn symbol(&mut self) -> Result<Token, Failure> {
        let character = self
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        let pair = match (character, self.peek()) {
            ('*', Some('*')) => Some(Symbol::CARET),
            ('=', Some('=')) => Some(Symbol::EQUAL),
            ('!', Some('=')) => Some(Symbol::NOT_EQUAL),
            ('<', Some('=')) => Some(Symbol::LESS_OR_EQUAL),
            ('>', Some('=')) => Some(Symbol::GREATER_OR_EQUAL),
            _ => None,
        };
        if let Some(symbol) = pair {
            self.bump();
            return Ok(Token::Symbol(symbol));
        }
        Ok(Token::Symbol(Symbol::try_from(character)?))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Precedence(u8);

impl Precedence {
    const LOWEST: Self = Self(0);
    const COMPARISON: Self = Self(5);
    const ADDITION: Self = Self(10);
    const MULTIPLICATION: Self = Self(20);
    const PREFIX: Self = Self(25);
    const POWER: Self = Self(30);
    const POSTFIX: Self = Self(40);

    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Associativity {
    Left,
    Right,
    None,
}

struct Arguments<'a>(&'a [Value]);

impl<'a> Arguments<'a> {
    fn new(values: &'a [Value]) -> Self {
        Self(values)
    }

    fn expect(&self, name: &Text, arity: Arity) -> Result<(), Failure> {
        if arity.accepts(self.0) {
            Ok(())
        } else {
            Err(Failure::new(format!("{} expects {}", name.as_ref(), arity)))
        }
    }

    fn number(&self, index: usize) -> Result<Number, Failure> {
        self.0
            .get(index)
            .and_then(|value| value.cast::<Number>())
            .copied()
            .ok_or_else(|| Failure::new("expected a number"))
    }

    fn truth(&self, index: usize) -> Result<bool, Failure> {
        self.0
            .get(index)
            .and_then(|value| value.cast::<Truth>())
            .map(|truth| truth.value())
            .ok_or_else(|| Failure::new("expected true or false"))
    }

    fn numbers(&self) -> Result<Vec<Number>, Failure> {
        self.0
            .iter()
            .map(|value| {
                value
                    .cast::<Number>()
                    .copied()
                    .ok_or_else(|| Failure::new("expected a number"))
            })
            .collect()
    }
}

impl<'a> std::ops::Deref for Arguments<'a> {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
struct Arity {
    minimum: usize,
    maximum: Option<usize>,
}

impl Arity {
    fn exactly(count: usize) -> Self {
        Self {
            minimum: count,
            maximum: Some(count),
        }
    }

    fn between(minimum: usize, maximum: usize) -> Self {
        Self {
            minimum,
            maximum: Some(maximum),
        }
    }

    fn at_least(minimum: usize) -> Self {
        Self {
            minimum,
            maximum: None,
        }
    }

    fn accepts(self, values: &[Value]) -> bool {
        values.len() >= self.minimum
            && self
                .maximum
                .is_none_or(|maximum| values.len() <= maximum)
    }

    fn counted(count: usize) -> String {
        format!("{count} argument{}", if count == 1 { "" } else { "s" })
    }
}

impl std::fmt::Display for Arity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.maximum {
            Some(maximum) if maximum == self.minimum => {
                write!(formatter, "{}", Self::counted(self.minimum))
            }
            Some(maximum) => write!(
                formatter,
                "{} or {}",
                self.minimum,
                Self::counted(maximum)
            ),
            None => write!(formatter, "at least {}", Self::counted(self.minimum)),
        }
    }
}

struct Operator {
    function: Function,
    precedence: Precedence,
    associativity: Associativity,
}

impl Operator {
    fn new(callable: impl Callable, precedence: Precedence, associativity: Associativity) -> Self {
        Self {
            function: Function::new(callable),
            precedence,
            associativity,
        }
    }
}

struct Positive;

impl Callable for Positive {
    fn name(&self) -> Text {
        Text::new("positive")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(arguments.number(0)?))
    }
}

struct Negation;

impl Callable for Negation {
    fn name(&self) -> Text {
        Text::new("negate")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(-arguments.number(0)?.value())))
    }
}

struct Addition;

impl Callable for Addition {
    fn name(&self) -> Text {
        Text::new("add")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        if let (Some(left), Some(right)) =
            (values[0].cast::<Key>(), values[1].cast::<Key>())
        {
            return Ok(Value::new(left.combine(right)?));
        }
        if let (Some(key), Some(number)) =
            (values[0].cast::<Key>(), values[1].cast::<Number>())
        {
            return Self::digit(key, number);
        }
        if let (Some(number), Some(key)) =
            (values[0].cast::<Number>(), values[1].cast::<Key>())
        {
            return Self::digit(key, number);
        }
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() + right.value())))
    }
}

impl Addition {
    fn digit(key: &Key, number: &Number) -> Result<Value, Failure> {
        let digit = number.value();
        if digit.fract() != 0.0 || !(0.0..=9.0).contains(&digit) {
            return Err(Failure::new("a key combines with a single digit"));
        }
        let digit = Key::new(
            element::Modifiers::NONE,
            Text::new(format!("{}", digit as u8)),
        );
        Ok(Value::new(key.combine(&digit)?))
    }
}

struct Subtraction;

impl Callable for Subtraction {
    fn name(&self) -> Text {
        Text::new("subtract")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() - right.value())))
    }
}

struct Multiplication;

impl Callable for Multiplication {
    fn name(&self) -> Text {
        Text::new("multiply")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() * right.value())))
    }
}

struct Division;

impl Callable for Division {
    fn name(&self) -> Text {
        Text::new("divide")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        if right.value() == 0.0 {
            return Err(Failure::new("division by zero"));
        }
        Ok(Value::new(Number::new(left.value() / right.value())))
    }
}

struct Modulo;

impl Callable for Modulo {
    fn name(&self) -> Text {
        Text::new("mod")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        if right.value() == 0.0 {
            return Err(Failure::new("remainder by zero"));
        }
        Ok(Value::new(Number::new(left.value() % right.value())))
    }
}

struct Power;

impl Callable for Power {
    fn name(&self) -> Text {
        Text::new("pow")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value().powf(right.value()))))
    }
}

struct Factorial;

impl Callable for Factorial {
    fn name(&self) -> Text {
        Text::new("factorial")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if value < 0.0 || value.fract() != 0.0 {
            return Err(Failure::new("factorial requires a non-negative integer"));
        }
        if value > 170.0 {
            return Err(Failure::new("factorial is too large"));
        }
        let mut result = 1.0;
        for number in 2..=(value as u32) {
            result *= number as f64;
        }
        Ok(Value::new(Number::new(result)))
    }
}

struct Percentage;

impl Callable for Percentage {
    fn name(&self) -> Text {
        Text::new("percentage")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value() / 100.0)))
    }
}

struct LogicalNot;

impl Callable for LogicalNot {
    fn name(&self) -> Text {
        Text::new("not")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Truth::new(!arguments.truth(0)?)))
    }
}

struct LogicalAnd;

impl Callable for LogicalAnd {
    fn name(&self) -> Text {
        Text::new("and")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        Ok(Value::new(Truth::new(
            arguments.truth(0)? && arguments.truth(1)?,
        )))
    }
}

struct LogicalOr;

impl Callable for LogicalOr {
    fn name(&self) -> Text {
        Text::new("or")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        Ok(Value::new(Truth::new(
            arguments.truth(0)? || arguments.truth(1)?,
        )))
    }
}

struct Comparison {
    name: &'static str,
    compare: fn(f64, f64) -> bool,
}

const COMPARISONS: [(Symbol, &str, fn(f64, f64) -> bool); 4] = [
    (Symbol::LESS, "less", |left, right| left < right),
    (Symbol::GREATER, "greater", |left, right| left > right),
    (Symbol::LESS_OR_EQUAL, "less_or_equal", |left, right| {
        left <= right
    }),
    (
        Symbol::GREATER_OR_EQUAL,
        "greater_or_equal",
        |left, right| left >= right,
    ),
];

impl Callable for Comparison {
    fn name(&self) -> Text {
        Text::new(self.name)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Truth::new((self.compare)(
            left.value(),
            right.value(),
        ))))
    }
}

struct Equality;

impl Callable for Equality {
    fn name(&self) -> Text {
        Text::new("equal")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        Arguments::new(values).expect(&self.name(), Arity::exactly(2))?;
        let same = values[0]
            .equals(&values[1])
            .ok_or_else(|| Failure::new("these values cannot be compared"))?;
        Ok(Value::new(Truth::new(same)))
    }
}

struct Inequality;

impl Callable for Inequality {
    fn name(&self) -> Text {
        Text::new("not_equal")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        Arguments::new(values).expect(&self.name(), Arity::exactly(2))?;
        let same = values[0]
            .equals(&values[1])
            .ok_or_else(|| Failure::new("these values cannot be compared"))?;
        Ok(Value::new(Truth::new(!same)))
    }
}

struct Conditional;

impl Callable for Conditional {
    fn name(&self) -> Text {
        Text::new("if")
    }

    fn lazy(&self) -> bool {
        true
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(3))?;
        let condition = values[0]
            .cast::<Function>()
            .ok_or_else(|| Failure::new("if requires expressions"))?
            .call(&[])?;
        let condition = condition
            .cast::<Truth>()
            .map(|truth| truth.value())
            .ok_or_else(|| Failure::new("if requires a condition"))?;
        let chosen = if condition { 1 } else { 2 };
        let branch = values[chosen]
            .cast::<Function>()
            .ok_or_else(|| Failure::new("if requires expressions"))?;
        branch.call(&[])
    }
}

struct Deferred {
    tokens: Vec<(Token, std::ops::Range<usize>)>,
    variables: std::rc::Rc<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Callable for Deferred {
    fn name(&self) -> Text {
        Text::new("expression")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        Arguments::new(values).expect(&self.name(), Arity::exactly(0))?;
        let session = Session {
            variables: std::rc::Rc::clone(&self.variables),
            language: std::rc::Rc::clone(&self.language),
            depth: std::rc::Rc::clone(&self.depth),
        };
        Parser::new(
            Tokens::new(self.tokens.clone()),
            &session.language.operators,
            &session,
        )
        .value()
    }
}

struct Sine;

impl Callable for Sine {
    fn name(&self) -> Text {
        Text::new("sin")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().sin())))
    }
}

struct Cosine;

impl Callable for Cosine {
    fn name(&self) -> Text {
        Text::new("cos")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().cos())))
    }
}

struct Tangent;

impl Callable for Tangent {
    fn name(&self) -> Text {
        Text::new("tan")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().tan())))
    }
}

struct Arcsine;

impl Callable for Arcsine {
    fn name(&self) -> Text {
        Text::new("asin")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::new(
                "arcsine requires a number between minus one and one",
            ));
        }
        Ok(Value::new(Number::new(value.asin())))
    }
}

struct Arccosine;

impl Callable for Arccosine {
    fn name(&self) -> Text {
        Text::new("acos")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::new(
                "arccosine requires a number between minus one and one",
            ));
        }
        Ok(Value::new(Number::new(value.acos())))
    }
}

struct Arctangent;

impl Callable for Arctangent {
    fn name(&self) -> Text {
        Text::new("atan")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        Ok(Value::new(Number::new(value.atan())))
    }
}

struct Arctangent2;

impl Callable for Arctangent2 {
    fn name(&self) -> Text {
        Text::new("atan2")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value().atan2(right.value()))))
    }
}

struct HyperbolicSine;

impl Callable for HyperbolicSine {
    fn name(&self) -> Text {
        Text::new("sinh")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().sinh())))
    }
}

struct HyperbolicCosine;

impl Callable for HyperbolicCosine {
    fn name(&self) -> Text {
        Text::new("cosh")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().cosh())))
    }
}

struct HyperbolicTangent;

impl Callable for HyperbolicTangent {
    fn name(&self) -> Text {
        Text::new("tanh")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().tanh())))
    }
}

struct SquareRoot;

impl Callable for SquareRoot {
    fn name(&self) -> Text {
        Text::new("sqrt")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if value < 0.0 {
            return Err(Failure::new(
                "square root requires a non-negative number",
            ));
        }
        Ok(Value::new(Number::new(value.sqrt())))
    }
}

struct CubeRoot;

impl Callable for CubeRoot {
    fn name(&self) -> Text {
        Text::new("cbrt")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().cbrt())))
    }
}

struct Absolute;

impl Callable for Absolute {
    fn name(&self) -> Text {
        Text::new("abs")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().abs())))
    }
}

struct NaturalLogarithm;

impl Callable for NaturalLogarithm {
    fn name(&self) -> Text {
        Text::new("ln")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        Ok(Value::new(Number::new(value.ln())))
    }
}

struct Logarithm;

impl Callable for Logarithm {
    fn name(&self) -> Text {
        Text::new("log")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::between(1, 2))?;
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        let result = if arguments.len() == 1 {
            value.log10()
        } else {
            let base = arguments.number(1)?.value();
            if base <= 0.0 || base == 1.0 {
                return Err(Failure::new(
                    "logarithm base must be positive and not one",
                ));
            }
            value.log(base)
        };
        Ok(Value::new(Number::new(result)))
    }
}

struct BinaryLogarithm;

impl Callable for BinaryLogarithm {
    fn name(&self) -> Text {
        Text::new("log2")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        Ok(Value::new(Number::new(value.log2())))
    }
}

struct DecimalLogarithm;

impl Callable for DecimalLogarithm {
    fn name(&self) -> Text {
        Text::new("log10")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        Ok(Value::new(Number::new(value.log10())))
    }
}

struct Exponential;

impl Callable for Exponential {
    fn name(&self) -> Text {
        Text::new("exp")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().exp())))
    }
}

struct Floor;

impl Callable for Floor {
    fn name(&self) -> Text {
        Text::new("floor")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().floor())))
    }
}

struct Ceiling;

impl Callable for Ceiling {
    fn name(&self) -> Text {
        Text::new("ceil")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().ceil())))
    }
}

struct Round;

impl Callable for Round {
    fn name(&self) -> Text {
        Text::new("round")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::between(1, 2))?;
        let value = arguments.number(0)?.value();
        if arguments.len() == 1 {
            return Ok(Value::new(Number::new(value.round())));
        }
        let places = arguments.number(1)?.value();
        if places < 0.0 || places.fract() != 0.0 || places > 15.0 {
            return Err(Failure::new(
                "round requires a whole number of places between zero and fifteen",
            ));
        }
        let factor = 10.0_f64.powi(places as i32);
        Ok(Value::new(Number::new((value * factor).round() / factor)))
    }
}

struct Truncate;

impl Callable for Truncate {
    fn name(&self) -> Text {
        Text::new("trunc")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        Ok(Value::new(Number::new(arguments.number(0)?.value().trunc())))
    }
}

struct Sign;

impl Callable for Sign {
    fn name(&self) -> Text {
        Text::new("sign")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(1))?;
        let value = arguments.number(0)?.value();
        let sign = if value == 0.0 { 0.0 } else { value.signum() };
        Ok(Value::new(Number::new(sign)))
    }
}

struct Minimum;

impl Callable for Minimum {
    fn name(&self) -> Text {
        Text::new("min")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::at_least(1))?;
        let numbers = arguments.numbers()?;
        let minimum = numbers
            .iter()
            .map(|number| number.value())
            .fold(f64::INFINITY, f64::min);
        Ok(Value::new(Number::new(minimum)))
    }
}

struct Maximum;

impl Callable for Maximum {
    fn name(&self) -> Text {
        Text::new("max")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::at_least(1))?;
        let numbers = arguments.numbers()?;
        let maximum = numbers
            .iter()
            .map(|number| number.value())
            .fold(f64::NEG_INFINITY, f64::max);
        Ok(Value::new(Number::new(maximum)))
    }
}

struct Hypotenuse;

impl Callable for Hypotenuse {
    fn name(&self) -> Text {
        Text::new("hypot")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(2))?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(
            left.value().hypot(right.value()),
        )))
    }
}

struct Operators {
    prefixes: std::collections::HashMap<Symbol, Operator>,
    infixes: std::collections::HashMap<Symbol, Operator>,
    postfixes: std::collections::HashMap<Symbol, Operator>,
}

impl Operators {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut prefixes = HashMap::new();
        prefixes.insert(
            Symbol::PLUS,
            Operator::new(Positive, Precedence::PREFIX, Associativity::None),
        );
        prefixes.insert(
            Symbol::MINUS,
            Operator::new(Negation, Precedence::PREFIX, Associativity::None),
        );
        let mut infixes = HashMap::new();
        infixes.insert(
            Symbol::PLUS,
            Operator::new(Addition, Precedence::ADDITION, Associativity::Left),
        );
        infixes.insert(
            Symbol::MINUS,
            Operator::new(Subtraction, Precedence::ADDITION, Associativity::Left),
        );
        infixes.insert(
            Symbol::STAR,
            Operator::new(
                Multiplication,
                Precedence::MULTIPLICATION,
                Associativity::Left,
            ),
        );
        infixes.insert(
            Symbol::SLASH,
            Operator::new(Division, Precedence::MULTIPLICATION, Associativity::Left),
        );
        infixes.insert(
            Symbol::PERCENT,
            Operator::new(Modulo, Precedence::MULTIPLICATION, Associativity::Left),
        );
        infixes.insert(
            Symbol::CARET,
            Operator::new(Power, Precedence::POWER, Associativity::Right),
        );
        for (symbol, name, compare) in COMPARISONS {
            infixes.insert(
                symbol,
                Operator::new(
                    Comparison { name, compare },
                    Precedence::COMPARISON,
                    Associativity::Left,
                ),
            );
        }
        infixes.insert(
            Symbol::EQUAL,
            Operator::new(Equality, Precedence::COMPARISON, Associativity::Left),
        );
        infixes.insert(
            Symbol::NOT_EQUAL,
            Operator::new(Inequality, Precedence::COMPARISON, Associativity::Left),
        );
        let mut postfixes = HashMap::new();
        postfixes.insert(
            Symbol::BANG,
            Operator::new(Factorial, Precedence::POSTFIX, Associativity::None),
        );
        postfixes.insert(
            Symbol::PERCENT,
            Operator::new(Percentage, Precedence::POSTFIX, Associativity::None),
        );
        Self {
            prefixes,
            infixes,
            postfixes,
        }
    }

    fn prefix(&self, symbol: &Symbol) -> Option<&Operator> {
        self.prefixes.get(symbol)
    }

    fn infix(&self, symbol: &Symbol) -> Option<&Operator> {
        self.infixes.get(symbol)
    }

    fn postfix(&self, symbol: &Symbol) -> Option<&Operator> {
        self.postfixes.get(symbol)
    }

    fn multiplication(&self) -> Option<&Operator> {
        self.infix(&Symbol::STAR)
    }

    fn value(&self, symbol: &Symbol) -> Option<Value> {
        self.infix(symbol)
            .or_else(|| self.prefix(symbol))
            .or_else(|| self.postfix(symbol))
            .map(|operator| Value::new(operator.function.clone()))
    }
}

struct Constants {
    values: std::collections::HashMap<Identifier, Value>,
}

impl Constants {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut values = HashMap::new();
        for (name, number) in [
            ("pi", std::f64::consts::PI),
            ("tau", std::f64::consts::TAU),
            ("e", std::f64::consts::E),
            ("deg", std::f64::consts::PI / 180.0),
            ("phi", (1.0 + 5.0_f64.sqrt()) / 2.0),
            ("inf", f64::INFINITY),
        ] {
            values.insert(Identifier::new(name), Value::new(Number::new(number)));
        }
        values.insert(Identifier::new("true"), Value::new(Truth::new(true)));
        values.insert(Identifier::new("false"), Value::new(Truth::new(false)));
        Self { values }
    }

    fn find(&self, name: &Identifier) -> Option<Value> {
        self.values.get(name).cloned()
    }

    fn contains(&self, name: &Identifier) -> bool {
        self.values.contains_key(name)
    }

    fn names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = self
            .values
            .iter()
            .map(|(identifier, value)| Name {
                text: Text::new(identifier.to_string()),
                value: value.clone(),
            })
            .collect();
        names.sort_by(|left, right| left.text.as_ref().cmp(right.text.as_ref()));
        names
    }
}

struct Functions {
    entries: std::collections::HashMap<Identifier, Value>,
}

impl Functions {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut entries = HashMap::new();
        entries.insert(
            Identifier::new("positive"),
            Value::new(Function::new(Positive)),
        );
        entries.insert(
            Identifier::new("negate"),
            Value::new(Function::new(Negation)),
        );
        entries.insert(Identifier::new("add"), Value::new(Function::new(Addition)));
        entries.insert(
            Identifier::new("subtract"),
            Value::new(Function::new(Subtraction)),
        );
        entries.insert(
            Identifier::new("multiply"),
            Value::new(Function::new(Multiplication)),
        );
        entries.insert(
            Identifier::new("divide"),
            Value::new(Function::new(Division)),
        );
        entries.insert(Identifier::new("mod"), Value::new(Function::new(Modulo)));
        entries.insert(Identifier::new("pow"), Value::new(Function::new(Power)));
        entries.insert(
            Identifier::new("factorial"),
            Value::new(Function::new(Factorial)),
        );
        entries.insert(
            Identifier::new("percentage"),
            Value::new(Function::new(Percentage)),
        );
        entries.insert(Identifier::new("not"), Value::new(Function::new(LogicalNot)));
        entries.insert(Identifier::new("and"), Value::new(Function::new(LogicalAnd)));
        entries.insert(Identifier::new("or"), Value::new(Function::new(LogicalOr)));
        entries.insert(
            Identifier::new("equal"),
            Value::new(Function::new(Equality)),
        );
        entries.insert(
            Identifier::new("not_equal"),
            Value::new(Function::new(Inequality)),
        );
        entries.insert(
            Identifier::new("if"),
            Value::new(Function::new(Conditional)),
        );
        for (_, name, compare) in COMPARISONS {
            entries.insert(
                Identifier::new(name),
                Value::new(Function::new(Comparison { name, compare })),
            );
        }
        entries.insert(
            Identifier::new("sin"),
            Value::new(Function::new(Sine)),
        );
        entries.insert(
            Identifier::new("cos"),
            Value::new(Function::new(Cosine)),
        );
        entries.insert(
            Identifier::new("tan"),
            Value::new(Function::new(Tangent)),
        );
        entries.insert(
            Identifier::new("asin"),
            Value::new(Function::new(Arcsine)),
        );
        entries.insert(
            Identifier::new("acos"),
            Value::new(Function::new(Arccosine)),
        );
        entries.insert(
            Identifier::new("atan"),
            Value::new(Function::new(Arctangent)),
        );
        entries.insert(
            Identifier::new("atan2"),
            Value::new(Function::new(Arctangent2)),
        );
        entries.insert(
            Identifier::new("sinh"),
            Value::new(Function::new(HyperbolicSine)),
        );
        entries.insert(
            Identifier::new("cosh"),
            Value::new(Function::new(HyperbolicCosine)),
        );
        entries.insert(
            Identifier::new("tanh"),
            Value::new(Function::new(HyperbolicTangent)),
        );
        entries.insert(
            Identifier::new("sqrt"),
            Value::new(Function::new(SquareRoot)),
        );
        entries.insert(
            Identifier::new("cbrt"),
            Value::new(Function::new(CubeRoot)),
        );
        entries.insert(
            Identifier::new("abs"),
            Value::new(Function::new(Absolute)),
        );
        entries.insert(
            Identifier::new("ln"),
            Value::new(Function::new(NaturalLogarithm)),
        );
        entries.insert(
            Identifier::new("log"),
            Value::new(Function::new(Logarithm)),
        );
        entries.insert(
            Identifier::new("log2"),
            Value::new(Function::new(BinaryLogarithm)),
        );
        entries.insert(
            Identifier::new("log10"),
            Value::new(Function::new(DecimalLogarithm)),
        );
        entries.insert(
            Identifier::new("exp"),
            Value::new(Function::new(Exponential)),
        );
        entries.insert(Identifier::new("floor"), Value::new(Function::new(Floor)));
        entries.insert(
            Identifier::new("ceil"),
            Value::new(Function::new(Ceiling)),
        );
        entries.insert(Identifier::new("round"), Value::new(Function::new(Round)));
        entries.insert(
            Identifier::new("trunc"),
            Value::new(Function::new(Truncate)),
        );
        entries.insert(Identifier::new("sign"), Value::new(Function::new(Sign)));
        entries.insert(
            Identifier::new("min"),
            Value::new(Function::new(Minimum)),
        );
        entries.insert(
            Identifier::new("max"),
            Value::new(Function::new(Maximum)),
        );
        entries.insert(
            Identifier::new("hypot"),
            Value::new(Function::new(Hypotenuse)),
        );
        Self { entries }
    }

    fn find(&self, name: &Identifier) -> Option<Value> {
        self.entries.get(name).cloned()
    }

    fn contains(&self, name: &Identifier) -> bool {
        self.entries.contains_key(name)
    }

    fn names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = self
            .entries
            .iter()
            .map(|(identifier, value)| Name {
                text: Text::new(identifier.to_string()),
                value: value.clone(),
            })
            .collect();
        names.sort_by(|left, right| left.text.as_ref().cmp(right.text.as_ref()));
        names
    }
}

struct Variables {
    values: std::collections::HashMap<Identifier, Value>,
    parent: Option<std::rc::Rc<std::cell::RefCell<Variables>>>,
}

struct Language {
    constants: Constants,
    functions: Functions,
    operators: Operators,
}

impl Language {
    fn new() -> Self {
        Self {
            constants: Constants::new(),
            functions: Functions::new(),
            operators: Operators::new(),
        }
    }

    fn learn(&mut self, name: Identifier, value: Value) {
        self.functions.entries.insert(name, value);
    }
}

struct Session {
    variables: std::rc::Rc<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Session {
    fn new(language: std::rc::Rc<Language>) -> Self {
        Self {
            variables: std::rc::Rc::new(std::cell::RefCell::new(Variables {
                values: std::collections::HashMap::new(),
                parent: None,
            })),
            language,
            depth: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }

    fn lookup(&self, name: &Identifier) -> Result<Value, Failure> {
        if name.is_answer() {
            return self
                .variable(name)
                .ok_or_else(|| Failure::new("ans is not defined"));
        }
        if let Some(value) = self.variable(name) {
            return Ok(value);
        }
        if let Some(value) = self.language.functions.find(name) {
            return Ok(value);
        }
        if let Some(value) = self.language.constants.find(name) {
            return Ok(value);
        }
        Err(Failure::new(format!("unknown name {name}")))
    }

    fn variable(&self, name: &Identifier) -> Option<Value> {
        let mut current = Some(std::rc::Rc::clone(&self.variables));
        while let Some(scope) = current {
            let borrow = scope.borrow();
            if let Some(value) = borrow.values.get(name) {
                return Some(value.clone());
            }
            current = borrow.parent.clone();
        }
        None
    }

    fn assign(&self, name: Identifier, value: Value) -> Result<(), Failure> {
        if name.is_answer()
            || self.language.constants.contains(&name)
            || self.language.functions.contains(&name)
        {
            return Err(Failure::new(format!("{name} is reserved")));
        }
        self.variables.borrow_mut().values.insert(name, value);
        Ok(())
    }

    fn remember(&self, value: Value) {
        self.variables
            .borrow_mut()
            .values
            .insert(Identifier::new("ans"), value);
    }

    fn names(&self) -> Vec<Name> {
        let mut names = Vec::new();
        let mut current = Some(std::rc::Rc::clone(&self.variables));
        while let Some(scope) = current {
            let borrow = scope.borrow();
            for (identifier, value) in &borrow.values {
                if identifier.is_answer() {
                    continue;
                }
                names.push(Name {
                    text: Text::new(identifier.to_string()),
                    value: value.clone(),
                });
            }
            current = borrow.parent.clone();
        }
        names.sort_by(|left, right| left.text.as_ref().cmp(right.text.as_ref()));
        names.extend(self.language.functions.names());
        names.extend(self.language.constants.names());
        names
    }
}

pub struct Parameter {
    name: Text,
}

impl element::Element for Parameter {}

impl Parameter {
    fn new(name: &Identifier) -> Self {
        Self {
            name: Text::new(name.to_string()),
        }
    }

    pub fn name(&self) -> &Text {
        &self.name
    }
}

struct Defined {
    name: Identifier,
    parameters: Vec<Identifier>,
    body: Expression,
    scope: std::rc::Weak<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Defined {
    fn evaluate(&self, values: &[Value]) -> Result<Value, Failure> {
        let scope = self
            .scope
            .upgrade()
            .ok_or_else(|| Failure::new("the definition is out of scope"))?;
        let mut bound = std::collections::HashMap::new();
        for (parameter, value) in self.parameters.iter().zip(values.iter()) {
            bound.insert(parameter.clone(), value.clone());
        }
        let session = Session {
            variables: std::rc::Rc::new(std::cell::RefCell::new(Variables {
                values: bound,
                parent: Some(scope),
            })),
            language: std::rc::Rc::clone(&self.language),
            depth: std::rc::Rc::clone(&self.depth),
        };
        let mut lexer = Lexer::new(&self.body);
        let tokens = lexer.tokens()?;
        Parser::new(tokens, &session.language.operators, &session).value()
    }
}

impl Callable for Defined {
    fn name(&self) -> Text {
        Text::new(self.name.to_string())
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::exactly(self.parameters.len()))?;
        let depth = self.depth.get() + 1;
        if depth > 64 {
            return Err(Failure::new("recursion is too deep"));
        }
        self.depth.set(depth);
        let result = self.evaluate(values);
        self.depth.set(depth - 1);
        result
    }
}

struct Parser<'a> {
    tokens: Tokens,
    operators: &'a Operators,
    session: &'a Session,
    expression: Option<&'a Expression>,
    parts: Option<&'a mut Vec<Part>>,
}

impl<'a> Parser<'a> {
    fn new(tokens: Tokens, operators: &'a Operators, session: &'a Session) -> Self {
        Self {
            tokens,
            operators,
            session,
            expression: None,
            parts: None,
        }
    }

    fn reading(mut self, expression: &'a Expression, parts: &'a mut Vec<Part>) -> Self {
        self.expression = Some(expression);
        self.parts = Some(parts);
        self
    }

    fn record(&mut self, index: usize, value: &Value) {
        let expression = match self.expression {
            Some(expression) => expression,
            None => return,
        };
        let parts = match self.parts.as_mut() {
            Some(parts) => parts,
            None => return,
        };
        if let Some(span) = self.tokens.span(index) {
            parts.push(Part {
                span: expression.span(span),
                value: value.clone(),
            });
        }
    }

    fn value(&mut self) -> Result<Value, Failure> {
        let value = self.expression(Precedence::LOWEST)?;
        if self.tokens.is_empty() {
            return Ok(value);
        }
        let token = self
            .tokens
            .peek()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        Err(Failure::new(format!("unexpected {token}")))
    }

    fn expression(&mut self, minimum: Precedence) -> Result<Value, Failure> {
        let mut left = self.prefix()?;
        loop {
            let token = match self.tokens.peek() {
                Some(token) => token.clone(),
                None => break,
            };
            let index = self.tokens.position;
            if let Token::Symbol(symbol) = &token {
                let percentage_remainder = *symbol == Symbol::PERCENT
                    && self.tokens.peek_after(1).is_some_and(Token::starts_value);
                if !percentage_remainder {
                    if let Some(operator) = self.operators.postfix(symbol) {
                        let precedence = operator.precedence;
                        let function = operator.function.clone();
                        if precedence >= minimum {
                            self.tokens.bump();
                            self.record(index, &Value::new(function.clone()));
                            left = function.call(&[left])?;
                            continue;
                        }
                    }
                }
                if let Some(operator) = self.operators.infix(symbol) {
                    let precedence = operator.precedence;
                    let associativity = operator.associativity;
                    let function = operator.function.clone();
                    if precedence < minimum {
                        break;
                    }
                    self.tokens.bump();
                    self.record(index, &Value::new(function.clone()));
                    let next_minimum = match associativity {
                        Associativity::Right => precedence,
                        _ => precedence.next(),
                    };
                    let right = self.expression(next_minimum)?;
                    left = function.call(&[left, right])?;
                    continue;
                }
            }
            if token.starts_value() {
                let (precedence, function) = match self.operators.multiplication() {
                    Some(operator) => (operator.precedence, operator.function.clone()),
                    None => break,
                };
                if precedence < minimum {
                    break;
                }
                let right = self.expression(precedence.next())?;
                left = function.call(&[left, right])?;
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Value, Failure> {
        let index = self.tokens.position;
        let token = self
            .tokens
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        match token {
            Token::Number(number) => {
                let value = Value::new(number);
                self.record(index, &value);
                Ok(value)
            }
            Token::Key(key) => {
                let value = Value::new(key);
                self.record(index, &value);
                Ok(value)
            }
            Token::Name(name) => self.name(index, name),
            Token::LeftParenthesis => {
                let value = self.expression(Precedence::LOWEST)?;
                self.expect_right_parenthesis()?;
                Ok(value)
            }
            Token::Symbol(symbol) => {
                if self.prefix_applies() {
                    if let Some(operator) = self.operators.prefix(&symbol) {
                        let precedence = operator.precedence;
                        let function = operator.function.clone();
                        self.record(index, &Value::new(function.clone()));
                        let value = self.expression(precedence)?;
                        return function.call(&[value]);
                    }
                }
                let value = self
                    .operators
                    .value(&symbol)
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {symbol}")))?;
                self.record(index, &value);
                Ok(value)
            }
            other => Err(Failure::new(format!("unexpected {other}"))),
        }
    }

    fn prefix_applies(&self) -> bool {
        match self.tokens.peek() {
            Some(token) => {
                token.starts_value()
                    || matches!(token, Token::Symbol(symbol) if self.operators.prefix(symbol).is_some())
            }
            None => false,
        }
    }

    fn name(&mut self, index: usize, name: Identifier) -> Result<Value, Failure> {
        if matches!(self.tokens.peek(), Some(Token::LeftParenthesis)) {
            self.tokens.bump();
            let value = match self.session.lookup(&name) {
                Ok(value) => value,
                Err(failure) => {
                    self.record(index, &Value::new(Failure::new(failure.to_string())));
                    return Err(failure);
                }
            };
            self.record(index, &value);
            let function = value
                .cast::<Function>()
                .ok_or_else(|| Failure::new(format!("{name} is not a function")))?;
            let arguments = if function.lazy() {
                self.lazy_arguments()?
            } else {
                self.arguments()?
            };
            return function.call(&arguments);
        }
        match self.session.lookup(&name) {
            Ok(value) => {
                self.record(index, &value);
                Ok(value)
            }
            Err(failure) => {
                self.record(index, &Value::new(Failure::new(failure.to_string())));
                Err(failure)
            }
        }
    }

    fn lazy_arguments(&mut self) -> Result<Vec<Value>, Failure> {
        let arguments = self.tokens.lazy_arguments()?;
        Ok(arguments
            .into_iter()
            .map(|tokens| {
                Value::new(Function::new(Deferred {
                    tokens,
                    variables: std::rc::Rc::clone(&self.session.variables),
                    language: std::rc::Rc::clone(&self.session.language),
                    depth: std::rc::Rc::clone(&self.session.depth),
                }))
            })
            .collect())
    }

    fn arguments(&mut self) -> Result<Vec<Value>, Failure> {
        if matches!(self.tokens.peek(), Some(Token::RightParenthesis)) {
            self.tokens.bump();
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        loop {
            values.push(self.expression(Precedence::LOWEST)?);
            let token = self.tokens.peek().cloned();
            match token {
                Some(Token::Comma) => {
                    self.tokens.bump();
                }
                Some(Token::RightParenthesis) => {
                    self.tokens.bump();
                    break;
                }
                Some(token) => {
                    return Err(Failure::new(format!(
                        "expected ',' or ')', found {token}"
                    )))
                }
                None => return Err(Failure::new("expected ',' or ')'")),
            }
        }
        Ok(values)
    }

    fn expect_right_parenthesis(&mut self) -> Result<(), Failure> {
        match self.tokens.bump() {
            Some(Token::RightParenthesis) => Ok(()),
            Some(token) => Err(Failure::new(format!("expected ')', found {token}"))),
            None => Err(Failure::new("expected ')'")),
        }
    }
}

pub struct Calculator {
    installed: Vec<(Identifier, Value)>,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            installed: Vec::new(),
        }
    }

    pub fn install(&mut self, name: &str, function: Function) {
        self.installed
            .push((Identifier::new(name), Value::new(function)));
    }

    pub fn evaluate(&self, source: &Source) -> Readings {
        let mut language = Language::new();
        for (name, value) in &self.installed {
            language.learn(name.clone(), value.clone());
        }
        let session = Session::new(std::rc::Rc::new(language));
        let mut readings = Vec::with_capacity(source.line_count());
        readings.extend(source.lines().map(|line| self.evaluate_line(line, &session)));
        Readings::new(readings, session.names())
    }

    fn evaluate_line(&self, line: Line<'_>, session: &Session) -> Reading {
        match line.statement() {
            Ok(Statement::Empty) => Reading::without_parts(Value::new(Blank::new())),
            Ok(Statement::Expression(expression)) => {
                let reading = self.evaluate_expression(&expression, session);
                if reading.value().cast::<Failure>().is_none() {
                    session.remember(reading.value().clone());
                }
                reading
            }
            Ok(Statement::Assignment { name, span, expression }) => {
                let mut reading = self.evaluate_expression(&expression, session);
                if reading.value().cast::<Failure>().is_none() {
                    match session.assign(name, reading.value().clone()) {
                        Ok(()) => {
                            session.remember(reading.value().clone());
                            let value = reading.value().clone();
                            reading.prefix(vec![Part { span, value }]);
                        }
                        Err(failure) => reading.replace(Value::new(failure)),
                    }
                }
                reading
            }
            Ok(Statement::Definition {
                name,
                span,
                parameters,
                expression,
            }) => {
                let mut names = Vec::with_capacity(parameters.len());
                let mut parts = Vec::with_capacity(parameters.len() + 1);
                for (parameter, span) in parameters {
                    names.push(parameter.clone());
                    parts.push(Part {
                        span,
                        value: Value::new(Parameter::new(&parameter)),
                    });
                }
                let function = Value::new(Function::new(Defined {
                    name: name.clone(),
                    parameters: names,
                    body: expression,
                    scope: std::rc::Rc::downgrade(&session.variables),
                    language: std::rc::Rc::clone(&session.language),
                    depth: std::rc::Rc::clone(&session.depth),
                }));
                parts.insert(
                    0,
                    Part {
                        span,
                        value: function.clone(),
                    },
                );
                match session.assign(name, function.clone()) {
                    Ok(()) => {
                        session.remember(function.clone());
                        Reading::new(function, parts)
                    }
                    Err(failure) => Reading::without_parts(Value::new(failure)),
                }
            }
            Err(failure) => Reading::without_parts(Value::new(failure)),
        }
    }

    fn evaluate_expression(&self, expression: &Expression, session: &Session) -> Reading {
        let mut parts = Vec::new();
        let mut lexer = Lexer::new(expression);
        let result = match lexer.tokens() {
            Ok(tokens) => Parser::new(tokens, &session.language.operators, session)
                .reading(expression, &mut parts)
                .value(),
            Err(failure) => Err(failure),
        };
        let value = match result {
            Ok(value)
                if value
                    .cast::<Number>()
                    .is_some_and(|number| number.value().is_nan()) =>
            {
                Value::new(Failure::new("not a real number"))
            }
            Ok(value) => value,
            Err(failure) => Value::new(failure),
        };
        Reading::new(value, parts)
    }
}
