use element::{Blank, Callable, Failure, Function, Number, Text, Value};

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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Angle {
    Radians,
    Degrees,
}

impl Default for Angle {
    fn default() -> Self {
        Self::Radians
    }
}

impl std::fmt::Display for Angle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Radians => "RAD",
            Self::Degrees => "DEG",
        })
    }
}

impl std::str::FromStr for Angle {
    type Err = Failure;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "RAD" => Ok(Self::Radians),
            "DEG" => Ok(Self::Degrees),
            _ => Err(Failure::new("invalid angle")),
        }
    }
}

impl Angle {
    pub(crate) fn to_radians(self, value: Number) -> Number {
        match self {
            Self::Radians => value,
            Self::Degrees => Number::new(value.value() * std::f64::consts::PI / 180.0),
        }
    }

    pub(crate) fn from_radians(self, value: Number) -> Number {
        match self {
            Self::Radians => value,
            Self::Degrees => Number::new(value.value() * 180.0 / std::f64::consts::PI),
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Radians => Self::Degrees,
            Self::Degrees => Self::Radians,
        }
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
        if let Some((left, right)) = code.split_once('=') {
            let name = Identifier::try_from(left)?;
            let expression = Expression::new(right);
            if expression.is_empty() {
                return Err(Failure::new("missing expression"));
            }
            return Ok(Statement::Assignment { name, expression });
        }
        let expression = Expression::new(code);
        if expression.is_empty() {
            return Ok(Statement::Empty);
        }
        Ok(Statement::Expression(expression))
    }

    fn without_comment(&self) -> &'a str {
        self.text
            .split_once('#')
            .map_or(self.text, |(code, _)| code)
    }
}

#[derive(Debug)]
struct Expression(String);

impl Expression {
    fn new(text: &str) -> Self {
        let mut normalized = String::with_capacity(text.len());
        for character in text.chars() {
            match character {
                '\u{06F0}'..='\u{06F9}' => normalized.push(
                    char::from_u32('0' as u32 + (character as u32 - '\u{06F0}' as u32))
                        .unwrap_or('0'),
                ),
                '\u{0660}'..='\u{0669}' => normalized.push(
                    char::from_u32('0' as u32 + (character as u32 - '\u{0660}' as u32))
                        .unwrap_or('0'),
                ),
                '×' | '·' => normalized.push('*'),
                '÷' => normalized.push('/'),
                '−' | '–' | '—' => normalized.push('-'),
                '٫' => normalized.push('.'),
                '٪' => normalized.push('%'),
                'π' => normalized.push_str("pi"),
                'τ' => normalized.push_str("tau"),
                '∞' => normalized.push_str("inf"),
                '\u{200E}' | '\u{200F}' => {}
                _ => normalized.push(character),
            }
        }
        Self(normalized.trim().to_string())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Expression {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
enum Statement {
    Empty,
    Expression(Expression),
    Assignment {
        name: Identifier,
        expression: Expression,
    },
}

pub struct Results(Vec<Value>);

impl std::ops::Deref for Results {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<Value>> for Results {
    fn from(evaluations: Vec<Value>) -> Self {
        Self(evaluations)
    }
}

impl IntoIterator for Results {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Results {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Symbol(char);

impl Symbol {
    const PLUS: Self = Self('+');
    const MINUS: Self = Self('-');
    const STAR: Self = Self('*');
    const SLASH: Self = Self('/');
    const PERCENT: Self = Self('%');
    const CARET: Self = Self('^');
    const BANG: Self = Self('!');
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
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
            _ => Err(Failure::new(format!("unexpected character {character}"))),
        }
    }
}

#[derive(Clone, Debug)]
enum Token {
    Number(Number),
    Name(Identifier),
    Symbol(Symbol),
    LeftParenthesis,
    RightParenthesis,
    Comma,
}

impl Token {
    fn starts_value(&self) -> bool {
        matches!(
            self,
            Self::Number(_) | Self::Name(_) | Self::LeftParenthesis
        )
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(formatter, "{}", number.value()),
            Self::Name(name) => write!(formatter, "{name}"),
            Self::Symbol(symbol) => write!(formatter, "{symbol}"),
            Self::LeftParenthesis => formatter.write_str("("),
            Self::RightParenthesis => formatter.write_str(")"),
            Self::Comma => formatter.write_str(","),
        }
    }
}

#[derive(Debug)]
struct Tokens {
    items: Vec<Token>,
    position: usize,
}

impl Tokens {
    fn new(items: Vec<Token>) -> Self {
        Self { items, position: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.items.get(self.position)
    }

    fn peek_after(&self, offset: usize) -> Option<&Token> {
        self.items.get(self.position + offset)
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
}

struct Lexer {
    characters: Vec<char>,
    position: usize,
}

impl Lexer {
    fn new(expression: &Expression) -> Self {
        Self {
            characters: expression.as_ref().chars().collect(),
            position: 0,
        }
    }

    fn tokens(&mut self) -> Result<Tokens, Failure> {
        let mut items = Vec::new();
        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                self.bump();
                continue;
            }
            items.push(self.token()?);
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
                Ok(Token::LeftParenthesis)
            }
            ')' => {
                self.bump();
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
        Token::Name(Identifier::new(&text))
    }

    fn symbol(&mut self) -> Result<Token, Failure> {
        let character = self
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        if character == '*' && self.peek() == Some('*') {
            self.bump();
            return Ok(Token::Symbol(Symbol::CARET));
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arity {
    One,
    Two,
    OneOrTwo,
    Many,
}

impl Arity {
    fn accepts(self, values: &[Value]) -> bool {
        match self {
            Self::One => values.len() == 1,
            Self::Two => values.len() == 2,
            Self::OneOrTwo => values.len() == 1 || values.len() == 2,
            Self::Many => !values.is_empty(),
        }
    }
}

impl std::fmt::Display for Arity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::One => "one argument",
            Self::Two => "two arguments",
            Self::OneOrTwo => "one or two arguments",
            Self::Many => "at least one argument",
        })
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() + right.value())))
    }
}

struct Subtraction;

impl Callable for Subtraction {
    fn name(&self) -> Text {
        Text::new("subtract")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::Two)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
        Ok(Value::new(Number::new(arguments.number(0)?.value() / 100.0)))
    }
}

struct Sine {
    angle: Angle,
}

impl Callable for Sine {
    fn name(&self) -> Text {
        Text::new("sin")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let radians = self.angle.to_radians(arguments.number(0)?);
        Ok(Value::new(Number::new(radians.value().sin())))
    }
}

struct Cosine {
    angle: Angle,
}

impl Callable for Cosine {
    fn name(&self) -> Text {
        Text::new("cos")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let radians = self.angle.to_radians(arguments.number(0)?);
        Ok(Value::new(Number::new(radians.value().cos())))
    }
}

struct Tangent {
    angle: Angle,
}

impl Callable for Tangent {
    fn name(&self) -> Text {
        Text::new("tan")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let radians = self.angle.to_radians(arguments.number(0)?);
        Ok(Value::new(Number::new(radians.value().tan())))
    }
}

struct Arcsine {
    angle: Angle,
}

impl Callable for Arcsine {
    fn name(&self) -> Text {
        Text::new("asin")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::new(
                "arcsine requires a number between minus one and one",
            ));
        }
        Ok(Value::new(self.angle.from_radians(Number::new(value.asin()))))
    }
}

struct Arccosine {
    angle: Angle,
}

impl Callable for Arccosine {
    fn name(&self) -> Text {
        Text::new("acos")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::new(
                "arccosine requires a number between minus one and one",
            ));
        }
        Ok(Value::new(self.angle.from_radians(Number::new(value.acos()))))
    }
}

struct Arctangent {
    angle: Angle,
}

impl Callable for Arctangent {
    fn name(&self) -> Text {
        Text::new("atan")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
        let value = arguments.number(0)?.value();
        Ok(Value::new(self.angle.from_radians(Number::new(value.atan()))))
    }
}

struct Arctangent2 {
    angle: Angle,
}

impl Callable for Arctangent2 {
    fn name(&self) -> Text {
        Text::new("atan2")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::Two)?;
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        let radians = Number::new(left.value().atan2(right.value()));
        Ok(Value::new(self.angle.from_radians(radians)))
    }
}

struct HyperbolicSine;

impl Callable for HyperbolicSine {
    fn name(&self) -> Text {
        Text::new("sinh")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::OneOrTwo)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::OneOrTwo)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::One)?;
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
        arguments.expect(&self.name(), Arity::Many)?;
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
        arguments.expect(&self.name(), Arity::Many)?;
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
        arguments.expect(&self.name(), Arity::Two)?;
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
    values: std::collections::HashMap<Identifier, Number>,
}

impl Constants {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut values = HashMap::new();
        values.insert(Identifier::new("pi"), Number::new(std::f64::consts::PI));
        values.insert(Identifier::new("tau"), Number::new(std::f64::consts::TAU));
        values.insert(Identifier::new("e"), Number::new(std::f64::consts::E));
        values.insert(
            Identifier::new("phi"),
            Number::new((1.0 + 5.0_f64.sqrt()) / 2.0),
        );
        values.insert(Identifier::new("inf"), Number::new(f64::INFINITY));
        Self { values }
    }

    fn find(&self, name: &Identifier) -> Option<Number> {
        self.values.get(name).copied()
    }

    fn contains(&self, name: &Identifier) -> bool {
        self.values.contains_key(name)
    }
}

struct Functions {
    entries: std::collections::HashMap<Identifier, Value>,
}

impl Functions {
    fn new(angle: Angle) -> Self {
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
        entries.insert(
            Identifier::new("sin"),
            Value::new(Function::new(Sine { angle })),
        );
        entries.insert(
            Identifier::new("cos"),
            Value::new(Function::new(Cosine { angle })),
        );
        entries.insert(
            Identifier::new("tan"),
            Value::new(Function::new(Tangent { angle })),
        );
        entries.insert(
            Identifier::new("asin"),
            Value::new(Function::new(Arcsine { angle })),
        );
        entries.insert(
            Identifier::new("acos"),
            Value::new(Function::new(Arccosine { angle })),
        );
        entries.insert(
            Identifier::new("atan"),
            Value::new(Function::new(Arctangent { angle })),
        );
        entries.insert(
            Identifier::new("atan2"),
            Value::new(Function::new(Arctangent2 { angle })),
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
}

struct Session<'a> {
    variables: std::collections::HashMap<Identifier, Value>,
    answer: Option<Value>,
    constants: &'a Constants,
    functions: &'a Functions,
}

impl<'a> Session<'a> {
    fn new(constants: &'a Constants, functions: &'a Functions) -> Self {
        use std::collections::HashMap;
        Self {
            variables: HashMap::new(),
            answer: None,
            constants,
            functions,
        }
    }

    fn lookup(&self, name: &Identifier) -> Result<Value, Failure> {
        if name.is_answer() {
            return self
                .answer
                .clone()
                .ok_or_else(|| Failure::new("ans is not defined"));
        }
        if let Some(value) = self.functions.find(name) {
            return Ok(value);
        }
        if let Some(value) = self.constants.find(name) {
            return Ok(Value::new(value));
        }
        if let Some(value) = self.variables.get(name) {
            return Ok(value.clone());
        }
        Err(Failure::new(format!("unknown name {name}")))
    }

    fn assign(&mut self, name: Identifier, value: Value) -> Result<(), Failure> {
        if name.is_answer() || self.constants.contains(&name) || self.functions.contains(&name) {
            return Err(Failure::new(format!("{name} is reserved")));
        }
        self.variables.insert(name, value);
        Ok(())
    }

    fn remember(&mut self, value: Value) {
        self.answer = Some(value);
    }
}

struct Parser<'operators, 'session, 'data> {
    tokens: Tokens,
    operators: &'operators Operators,
    session: &'session mut Session<'data>,
}

impl<'operators, 'session, 'data> Parser<'operators, 'session, 'data> {
    fn new(
        tokens: Tokens,
        operators: &'operators Operators,
        session: &'session mut Session<'data>,
    ) -> Self {
        Self {
            tokens,
            operators,
            session,
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
            if let Token::Symbol(symbol) = &token {
                let percentage_remainder = *symbol == Symbol::PERCENT
                    && self.tokens.peek_after(1).is_some_and(Token::starts_value);
                if !percentage_remainder {
                    if let Some(operator) = self.operators.postfix(symbol) {
                        let precedence = operator.precedence;
                        let function = operator.function.clone();
                        if precedence >= minimum {
                            self.tokens.bump();
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
        let token = self
            .tokens
            .bump()
            .ok_or_else(|| Failure::new("unexpected end of expression"))?;
        match token {
            Token::Number(number) => Ok(Value::new(number)),
            Token::Name(name) => self.name(name),
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
                        let value = self.expression(precedence)?;
                        return function.call(&[value]);
                    }
                }
                self.operators
                    .value(&symbol)
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {symbol}")))
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

    fn name(&mut self, name: Identifier) -> Result<Value, Failure> {
        if matches!(self.tokens.peek(), Some(Token::LeftParenthesis)) {
            self.tokens.bump();
            let arguments = self.arguments()?;
            let function = self.session.lookup(&name)?;
            let function = function
                .cast::<Function>()
                .ok_or_else(|| Failure::new(format!("{name} is not a function")))?;
            return function.call(&arguments);
        }
        self.session.lookup(&name)
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
    constants: Constants,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            constants: Constants::new(),
        }
    }

    pub fn evaluate(&self, source: &Source, angle: Angle) -> Results {
        let functions = Functions::new(angle);
        let operators = Operators::new();
        let mut session = Session::new(&self.constants, &functions);
        let mut evaluations = Vec::with_capacity(source.line_count());
        evaluations.extend(
            source
                .lines()
                .map(|line| self.evaluate_line(line, &mut session, &operators)),
        );
        Results::from(evaluations)
    }

    fn evaluate_line(
        &self,
        line: Line<'_>,
        session: &mut Session<'_>,
        operators: &Operators,
    ) -> Value {
        match line.statement() {
            Ok(Statement::Empty) => Value::new(Blank::new()),
            Ok(Statement::Expression(expression)) => {
                match self.evaluate_expression(&expression, session, operators) {
                    Ok(value) => {
                        session.remember(value.clone());
                        value
                    }
                    Err(failure) => Value::new(failure),
                }
            }
            Ok(Statement::Assignment { name, expression }) => {
                match self.evaluate_expression(&expression, session, operators) {
                    Ok(value) => match session.assign(name, value.clone()) {
                        Ok(()) => {
                            session.remember(value.clone());
                            value
                        }
                        Err(failure) => Value::new(failure),
                    },
                    Err(failure) => Value::new(failure),
                }
            }
            Err(failure) => Value::new(failure),
        }
    }

    fn evaluate_expression(
        &self,
        expression: &Expression,
        session: &mut Session<'_>,
        operators: &Operators,
    ) -> Result<Value, Failure> {
        let mut lexer = Lexer::new(expression);
        let tokens = lexer.tokens()?;
        let value = Parser::new(tokens, operators, session).value()?;
        if value
            .cast::<Number>()
            .is_some_and(|number| number.value().is_nan())
        {
            return Err(Failure::new("not a real number"));
        }
        Ok(value)
    }
}
