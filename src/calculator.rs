#[derive(Debug)]
pub(crate) struct Presentation(String);

impl Presentation {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for Presentation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Presentation {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Presentation {
    fn from(text: String) -> Self {
        Self(text)
    }
}

#[derive(Debug)]
pub(crate) struct Failure(Presentation);

impl Failure {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let mut characters = message.chars();
        let capitalized = match characters.next() {
            Some(first) => first.to_uppercase().chain(characters).collect::<String>(),
            None => String::new(),
        };
        Self(Presentation::new(capitalized))
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0.as_ref())
    }
}

impl std::error::Error for Failure {}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Value(f64);

impl From<f64> for Value {
    fn from(number: f64) -> Self {
        Self(number)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.formatted(Some(',')))
    }
}

impl Value {
    pub(crate) fn plain(&self) -> Presentation {
        Presentation::new(self.formatted(None))
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    pub(crate) fn subtract(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }

    pub(crate) fn multiply(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    pub(crate) fn divide(self, other: Self) -> Result<Self, Failure> {
        if other.is_zero() {
            Err(Failure::new("division by zero"))
        } else {
            Ok(Self(self.0 / other.0))
        }
    }

    pub(crate) fn remainder(self, other: Self) -> Result<Self, Failure> {
        if other.is_zero() {
            Err(Failure::new("remainder by zero"))
        } else {
            Ok(Self(self.0 % other.0))
        }
    }

    pub(crate) fn power(self, other: Self) -> Self {
        Self(self.0.powf(other.0))
    }

    pub(crate) fn negate(self) -> Self {
        Self(-self.0)
    }

    pub(crate) fn percentage(self) -> Self {
        Self(self.0 / 100.0)
    }

    pub(crate) fn factorial(self) -> Result<Self, Failure> {
        if self.is_negative() || self.0.fract() != 0.0 {
            return Err(Failure::new("factorial requires a non-negative integer"));
        }
        if self.0 > 170.0 {
            return Err(Failure::new("factorial is too large"));
        }
        let count = self.0 as u32;
        let mut result = 1.0;
        for number in 2..=count {
            result *= number as f64;
        }
        Ok(Self(result))
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.0 < 0.0
    }

    pub(crate) fn is_nan(&self) -> bool {
        self.0.is_nan()
    }

    pub(crate) fn is_infinite(&self) -> bool {
        self.0.is_infinite()
    }

    fn formatted(&self, separator: Option<char>) -> String {
        if self.0.is_nan() {
            return String::from("NaN");
        }
        if self.is_infinite() {
            return if self.0.is_sign_positive() {
                String::from("∞")
            } else {
                String::from("-∞")
            };
        }
        if self.0 == 0.0 {
            return String::from("0");
        }
        let magnitude = self.0.abs();
        if magnitude >= 1.0e15 || magnitude < 1.0e-9 {
            return format!("{:e}", self.0);
        }
        let rounded = (self.0 * 1.0e12).round() / 1.0e12;
        if rounded == 0.0 {
            return String::from("0");
        }
        let mut text = format!("{:.12}", rounded);
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        match separator {
            Some(separator) => match text.split_once('.') {
                Some((integer, fraction)) => {
                    format!("{}{}{}", Self::group(integer), separator, fraction)
                }
                None => Self::group(&text),
            },
            None => text,
        }
    }

    fn group(integer: &str) -> String {
        let (sign, digits) = match integer.strip_prefix('-') {
            Some(digits) => ("-", digits),
            None => ("", integer),
        };
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (position, character) in digits.chars().enumerate() {
            if position > 0 && (digits.len() - position) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(character);
        }
        let mut result = String::with_capacity(sign.len() + grouped.len());
        result.push_str(sign);
        result.push_str(&grouped);
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Angle {
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
    pub(crate) fn to_radians(self, value: Value) -> Value {
        match self {
            Self::Radians => value,
            Self::Degrees => {
                value.multiply(Value::from(std::f64::consts::PI / 180.0))
            }
        }
    }

    pub(crate) fn from_radians(self, value: Value) -> Value {
        match self {
            Self::Radians => value,
            Self::Degrees => {
                value.multiply(Value::from(180.0 / std::f64::consts::PI))
            }
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Self::Radians => Self::Degrees,
            Self::Degrees => Self::Radians,
        }
    }
}

impl Value {
    pub(crate) fn absolute(self) -> Self {
        Self(self.0.abs())
    }

    pub(crate) fn square_root(self) -> Result<Self, Failure> {
        if self.is_negative() {
            Err(Failure::new("square root requires a non-negative number"))
        } else {
            Ok(Self(self.0.sqrt()))
        }
    }

    pub(crate) fn cube_root(self) -> Self {
        Self(self.0.cbrt())
    }

    pub(crate) fn natural_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.ln()))
        }
    }

    pub(crate) fn binary_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.log2()))
        }
    }

    pub(crate) fn decimal_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.log10()))
        }
    }

    pub(crate) fn logarithm(self, base: Self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        if base.0 <= 0.0 || base.0 == 1.0 {
            return Err(Failure::new("logarithm base must be positive and not one"));
        }
        Ok(Self(self.0.log(base.0)))
    }

    pub(crate) fn exponential(self) -> Self {
        Self(self.0.exp())
    }

    pub(crate) fn floor(self) -> Self {
        Self(self.0.floor())
    }

    pub(crate) fn ceiling(self) -> Self {
        Self(self.0.ceil())
    }

    pub(crate) fn round(self) -> Self {
        Self(self.0.round())
    }

    pub(crate) fn round_to(self, places: Self) -> Result<Self, Failure> {
        if places.0 < 0.0 || places.0.fract() != 0.0 || places.0 > 15.0 {
            return Err(Failure::new(
                "round requires a whole number of places between zero and fifteen",
            ));
        }
        let factor = 10.0_f64.powi(places.0 as i32);
        Ok(Self((self.0 * factor).round() / factor))
    }

    pub(crate) fn truncate(self) -> Self {
        Self(self.0.trunc())
    }

    pub(crate) fn sign(self) -> Self {
        if self.is_zero() {
            Self(0.0)
        } else {
            Self(self.0.signum())
        }
    }

    pub(crate) fn sine(self) -> Self {
        Self(self.0.sin())
    }

    pub(crate) fn cosine(self) -> Self {
        Self(self.0.cos())
    }

    pub(crate) fn tangent(self) -> Self {
        Self(self.0.tan())
    }

    pub(crate) fn arcsine(self) -> Result<Self, Failure> {
        if !(-1.0..=1.0).contains(&self.0) {
            Err(Failure::new("arcsine requires a number between minus one and one"))
        } else {
            Ok(Self(self.0.asin()))
        }
    }

    pub(crate) fn arccosine(self) -> Result<Self, Failure> {
        if !(-1.0..=1.0).contains(&self.0) {
            Err(Failure::new("arccosine requires a number between minus one and one"))
        } else {
            Ok(Self(self.0.acos()))
        }
    }

    pub(crate) fn arctangent(self) -> Self {
        Self(self.0.atan())
    }

    pub(crate) fn arctangent2(self, other: Self) -> Self {
        Self(self.0.atan2(other.0))
    }

    pub(crate) fn hyperbolic_sine(self) -> Self {
        Self(self.0.sinh())
    }

    pub(crate) fn hyperbolic_cosine(self) -> Self {
        Self(self.0.cosh())
    }

    pub(crate) fn hyperbolic_tangent(self) -> Self {
        Self(self.0.tanh())
    }

    pub(crate) fn minimum(values: &[Self]) -> Result<Self, Failure> {
        values
            .iter()
            .copied()
            .reduce(|left, right| Self(left.0.min(right.0)))
            .ok_or_else(|| Failure::new("minimum accepts at least one argument"))
    }

    pub(crate) fn maximum(values: &[Self]) -> Result<Self, Failure> {
        values
            .iter()
            .copied()
            .reduce(|left, right| Self(left.0.max(right.0)))
            .ok_or_else(|| Failure::new("maximum accepts at least one argument"))
    }

    pub(crate) fn hypotenuse(self, other: Self) -> Self {
        Self(self.0.hypot(other.0))
    }
}

#[derive(Debug)]
pub(crate) struct Source(String);

impl Source {
    pub(crate) fn new(text: String) -> Self {
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

#[derive(Debug)]
pub(crate) struct Results(Vec<Evaluation>);

impl std::ops::Deref for Results {
    type Target = [Evaluation];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<Evaluation>> for Results {
    fn from(evaluations: Vec<Evaluation>) -> Self {
        Self(evaluations)
    }
}

impl IntoIterator for Results {
    type Item = Evaluation;
    type IntoIter = std::vec::IntoIter<Evaluation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Results {
    type Item = &'a Evaluation;
    type IntoIter = std::slice::Iter<'a, Evaluation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug)]
pub(crate) enum Evaluation {
    Blank,
    Value(Value),
    Failure(Failure),
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
    Number(Value),
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
            Self::Number(value) => write!(formatter, "{value}"),
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
        Ok(Token::Number(Value::from(number)))
    }

    fn radix(&mut self, radix: u32) -> Result<Token, Failure> {
        let mut text = String::new();
        self.digits(radix, &mut text);
        if text.is_empty() {
            return Err(Failure::new("missing digits"));
        }
        let number = u128::from_str_radix(&text, radix)
            .map_err(|_| Failure::new("invalid number"))?;
        Ok(Token::Number(Value::from(number as f64)))
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

#[derive(Debug)]
struct Arguments(Vec<Value>);

impl Arguments {
    fn one(&self) -> Result<Value, Failure> {
        if self.0.len() != 1 {
            return Err(Failure::new("expected one argument"));
        }
        self.0
            .first()
            .copied()
            .ok_or_else(|| Failure::new("expected one argument"))
    }

    fn two(&self) -> Result<(Value, Value), Failure> {
        if self.0.len() != 2 {
            return Err(Failure::new("expected two arguments"));
        }
        Ok((self.0[0], self.0[1]))
    }
}

impl std::ops::Deref for Arguments {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<Value>> for Arguments {
    fn from(values: Vec<Value>) -> Self {
        Self(values)
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
    fn accepts(self, arguments: &Arguments) -> bool {
        match self {
            Self::One => arguments.len() == 1,
            Self::Two => arguments.len() == 2,
            Self::OneOrTwo => arguments.len() == 1 || arguments.len() == 2,
            Self::Many => !arguments.is_empty(),
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

trait Operator: std::fmt::Display {
    fn precedence(&self) -> Precedence;

    fn associativity(&self) -> Associativity;

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure>;
}

struct Positive;

impl std::fmt::Display for Positive {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("+")
    }
}

impl Operator for Positive {
    fn precedence(&self) -> Precedence {
        Precedence::PREFIX
    }

    fn associativity(&self) -> Associativity {
        Associativity::None
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        arguments.one()
    }
}

struct Negation;

impl std::fmt::Display for Negation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("-")
    }
}

impl Operator for Negation {
    fn precedence(&self) -> Precedence {
        Precedence::PREFIX
    }

    fn associativity(&self) -> Associativity {
        Associativity::None
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        Ok(arguments.one()?.negate())
    }
}

struct Addition;

impl std::fmt::Display for Addition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("+")
    }
}

impl Operator for Addition {
    fn precedence(&self) -> Precedence {
        Precedence::ADDITION
    }

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.add(right))
    }
}

struct Subtraction;

impl std::fmt::Display for Subtraction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("-")
    }
}

impl Operator for Subtraction {
    fn precedence(&self) -> Precedence {
        Precedence::ADDITION
    }

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.subtract(right))
    }
}

struct Multiplication;

impl std::fmt::Display for Multiplication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("*")
    }
}

impl Operator for Multiplication {
    fn precedence(&self) -> Precedence {
        Precedence::MULTIPLICATION
    }

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.multiply(right))
    }
}

struct Division;

impl std::fmt::Display for Division {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("/")
    }
}

impl Operator for Division {
    fn precedence(&self) -> Precedence {
        Precedence::MULTIPLICATION
    }

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        left.divide(right)
    }
}

struct RemainderOperator;

impl std::fmt::Display for RemainderOperator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("%")
    }
}

impl Operator for RemainderOperator {
    fn precedence(&self) -> Precedence {
        Precedence::MULTIPLICATION
    }

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        left.remainder(right)
    }
}

struct PowerOperator;

impl std::fmt::Display for PowerOperator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("^")
    }
}

impl Operator for PowerOperator {
    fn precedence(&self) -> Precedence {
        Precedence::POWER
    }

    fn associativity(&self) -> Associativity {
        Associativity::Right
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.power(right))
    }
}

struct Factorial;

impl std::fmt::Display for Factorial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("!")
    }
}

impl Operator for Factorial {
    fn precedence(&self) -> Precedence {
        Precedence::POSTFIX
    }

    fn associativity(&self) -> Associativity {
        Associativity::None
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        arguments.one()?.factorial()
    }
}

struct Percentage;

impl std::fmt::Display for Percentage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("%")
    }
}

impl Operator for Percentage {
    fn precedence(&self) -> Precedence {
        Precedence::POSTFIX
    }

    fn associativity(&self) -> Associativity {
        Associativity::None
    }

    fn apply(&self, arguments: &Arguments) -> Result<Value, Failure> {
        Ok(arguments.one()?.percentage())
    }
}

struct Operators {
    prefixes: std::collections::HashMap<Symbol, Box<dyn Operator>>,
    infixes: std::collections::HashMap<Symbol, Box<dyn Operator>>,
    postfixes: std::collections::HashMap<Symbol, Box<dyn Operator>>,
}

impl Operators {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut prefixes: HashMap<Symbol, Box<dyn Operator>> = HashMap::new();
        prefixes.insert(Symbol::PLUS, Box::new(Positive));
        prefixes.insert(Symbol::MINUS, Box::new(Negation));
        let mut infixes: HashMap<Symbol, Box<dyn Operator>> = HashMap::new();
        infixes.insert(Symbol::PLUS, Box::new(Addition));
        infixes.insert(Symbol::MINUS, Box::new(Subtraction));
        infixes.insert(Symbol::STAR, Box::new(Multiplication));
        infixes.insert(Symbol::SLASH, Box::new(Division));
        infixes.insert(Symbol::PERCENT, Box::new(RemainderOperator));
        infixes.insert(Symbol::CARET, Box::new(PowerOperator));
        let mut postfixes: HashMap<Symbol, Box<dyn Operator>> = HashMap::new();
        postfixes.insert(Symbol::BANG, Box::new(Factorial));
        postfixes.insert(Symbol::PERCENT, Box::new(Percentage));
        Self {
            prefixes,
            infixes,
            postfixes,
        }
    }

    fn prefix(&self, symbol: &Symbol) -> Option<&dyn Operator> {
        self.prefixes.get(symbol).map(|operator| operator.as_ref())
    }

    fn infix(&self, symbol: &Symbol) -> Option<&dyn Operator> {
        self.infixes.get(symbol).map(|operator| operator.as_ref())
    }

    fn postfix(&self, symbol: &Symbol) -> Option<&dyn Operator> {
        self.postfixes.get(symbol).map(|operator| operator.as_ref())
    }

    fn multiplication(&self) -> Option<&dyn Operator> {
        self.infix(&Symbol::STAR)
    }
}

trait Calculation: std::fmt::Display {
    fn arity(&self) -> Arity;

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure>;

    fn name(&self) -> Identifier {
        Identifier::new(&self.to_string())
    }

    fn validate(&self, arguments: &Arguments) -> Result<(), Failure> {
        if self.arity().accepts(arguments) {
            Ok(())
        } else {
            Err(Failure::new(format!(
                "{} expects {}",
                self.name(),
                self.arity()
            )))
        }
    }
}

struct Sine;

impl std::fmt::Display for Sine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sin")
    }
}

impl Calculation for Sine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.to_radians(arguments.one()?).sine())
    }
}

struct Cosine;

impl std::fmt::Display for Cosine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("cos")
    }
}

impl Calculation for Cosine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.to_radians(arguments.one()?).cosine())
    }
}

struct Tangent;

impl std::fmt::Display for Tangent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("tan")
    }
}

impl Calculation for Tangent {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.to_radians(arguments.one()?).tangent())
    }
}

struct Arcsine;

impl std::fmt::Display for Arcsine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("asin")
    }
}

impl Calculation for Arcsine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.from_radians(arguments.one()?.arcsine()?))
    }
}

struct Arccosine;

impl std::fmt::Display for Arccosine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("acos")
    }
}

impl Calculation for Arccosine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.from_radians(arguments.one()?.arccosine()?))
    }
}

struct Arctangent;

impl std::fmt::Display for Arctangent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("atan")
    }
}

impl Calculation for Arctangent {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        Ok(angle.from_radians(arguments.one()?.arctangent()))
    }
}

struct Arctangent2;

impl std::fmt::Display for Arctangent2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("atan2")
    }
}

impl Calculation for Arctangent2 {
    fn arity(&self) -> Arity {
        Arity::Two
    }

    fn calculate(&self, arguments: &Arguments, angle: Angle) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(angle.from_radians(left.arctangent2(right)))
    }
}

struct HyperbolicSine;

impl std::fmt::Display for HyperbolicSine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sinh")
    }
}

impl Calculation for HyperbolicSine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.hyperbolic_sine())
    }
}

struct HyperbolicCosine;

impl std::fmt::Display for HyperbolicCosine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("cosh")
    }
}

impl Calculation for HyperbolicCosine {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.hyperbolic_cosine())
    }
}

struct HyperbolicTangent;

impl std::fmt::Display for HyperbolicTangent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("tanh")
    }
}

impl Calculation for HyperbolicTangent {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.hyperbolic_tangent())
    }
}

struct SquareRoot;

impl std::fmt::Display for SquareRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sqrt")
    }
}

impl Calculation for SquareRoot {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        arguments.one()?.square_root()
    }
}

struct CubeRoot;

impl std::fmt::Display for CubeRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("cbrt")
    }
}

impl Calculation for CubeRoot {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.cube_root())
    }
}

struct Absolute;

impl std::fmt::Display for Absolute {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("abs")
    }
}

impl Calculation for Absolute {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.absolute())
    }
}

struct NaturalLogarithm;

impl std::fmt::Display for NaturalLogarithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ln")
    }
}

impl Calculation for NaturalLogarithm {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        arguments.one()?.natural_logarithm()
    }
}

struct Logarithm;

impl std::fmt::Display for Logarithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("log")
    }
}

impl Calculation for Logarithm {
    fn arity(&self) -> Arity {
        Arity::OneOrTwo
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        match arguments.len() {
            1 => arguments.one()?.decimal_logarithm(),
            2 => {
                let (value, base) = arguments.two()?;
                value.logarithm(base)
            }
            _ => Err(Failure::new("log expects one or two arguments")),
        }
    }
}

struct BinaryLogarithm;

impl std::fmt::Display for BinaryLogarithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("log2")
    }
}

impl Calculation for BinaryLogarithm {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        arguments.one()?.binary_logarithm()
    }
}

struct DecimalLogarithm;

impl std::fmt::Display for DecimalLogarithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("log10")
    }
}

impl Calculation for DecimalLogarithm {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        arguments.one()?.decimal_logarithm()
    }
}

struct Exponential;

impl std::fmt::Display for Exponential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("exp")
    }
}

impl Calculation for Exponential {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.exponential())
    }
}

struct Floor;

impl std::fmt::Display for Floor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("floor")
    }
}

impl Calculation for Floor {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.floor())
    }
}

struct Ceiling;

impl std::fmt::Display for Ceiling {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ceil")
    }
}

impl Calculation for Ceiling {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.ceiling())
    }
}

struct Round;

impl std::fmt::Display for Round {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("round")
    }
}

impl Calculation for Round {
    fn arity(&self) -> Arity {
        Arity::OneOrTwo
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        match arguments.len() {
            1 => Ok(arguments.one()?.round()),
            2 => {
                let (value, places) = arguments.two()?;
                value.round_to(places)
            }
            _ => Err(Failure::new("round expects one or two arguments")),
        }
    }
}

struct Truncate;

impl std::fmt::Display for Truncate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("trunc")
    }
}

impl Calculation for Truncate {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.truncate())
    }
}

struct Sign;

impl std::fmt::Display for Sign {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sign")
    }
}

impl Calculation for Sign {
    fn arity(&self) -> Arity {
        Arity::One
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Ok(arguments.one()?.sign())
    }
}

struct Minimum;

impl std::fmt::Display for Minimum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("min")
    }
}

impl Calculation for Minimum {
    fn arity(&self) -> Arity {
        Arity::Many
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Value::minimum(arguments)
    }
}

struct Maximum;

impl std::fmt::Display for Maximum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("max")
    }
}

impl Calculation for Maximum {
    fn arity(&self) -> Arity {
        Arity::Many
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        Value::maximum(arguments)
    }
}

struct Power;

impl std::fmt::Display for Power {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("pow")
    }
}

impl Calculation for Power {
    fn arity(&self) -> Arity {
        Arity::Two
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.power(right))
    }
}

struct Remainder;

impl std::fmt::Display for Remainder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("mod")
    }
}

impl Calculation for Remainder {
    fn arity(&self) -> Arity {
        Arity::Two
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        left.remainder(right)
    }
}

struct Hypotenuse;

impl std::fmt::Display for Hypotenuse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("hypot")
    }
}

impl Calculation for Hypotenuse {
    fn arity(&self) -> Arity {
        Arity::Two
    }

    fn calculate(&self, arguments: &Arguments, _: Angle) -> Result<Value, Failure> {
        let (left, right) = arguments.two()?;
        Ok(left.hypotenuse(right))
    }
}

struct Constants {
    values: std::collections::HashMap<Identifier, Value>,
}

impl Constants {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut values = HashMap::new();
        values.insert(Identifier::new("pi"), Value::from(std::f64::consts::PI));
        values.insert(Identifier::new("tau"), Value::from(std::f64::consts::TAU));
        values.insert(Identifier::new("e"), Value::from(std::f64::consts::E));
        values.insert(
            Identifier::new("phi"),
            Value::from((1.0 + 5.0_f64.sqrt()) / 2.0),
        );
        values.insert(Identifier::new("inf"), Value::from(f64::INFINITY));
        Self { values }
    }

    fn find(&self, name: &Identifier) -> Option<Value> {
        self.values.get(name).copied()
    }

    fn contains(&self, name: &Identifier) -> bool {
        self.values.contains_key(name)
    }
}

struct Calculations {
    entries: std::collections::HashMap<Identifier, Box<dyn Calculation>>,
}

impl Calculations {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut calculations = Self {
            entries: HashMap::new(),
        };
        calculations.insert(Box::new(Sine));
        calculations.insert(Box::new(Cosine));
        calculations.insert(Box::new(Tangent));
        calculations.insert(Box::new(Arcsine));
        calculations.insert(Box::new(Arccosine));
        calculations.insert(Box::new(Arctangent));
        calculations.insert(Box::new(Arctangent2));
        calculations.insert(Box::new(HyperbolicSine));
        calculations.insert(Box::new(HyperbolicCosine));
        calculations.insert(Box::new(HyperbolicTangent));
        calculations.insert(Box::new(SquareRoot));
        calculations.insert(Box::new(CubeRoot));
        calculations.insert(Box::new(Absolute));
        calculations.insert(Box::new(NaturalLogarithm));
        calculations.insert(Box::new(Logarithm));
        calculations.insert(Box::new(BinaryLogarithm));
        calculations.insert(Box::new(DecimalLogarithm));
        calculations.insert(Box::new(Exponential));
        calculations.insert(Box::new(Floor));
        calculations.insert(Box::new(Ceiling));
        calculations.insert(Box::new(Round));
        calculations.insert(Box::new(Truncate));
        calculations.insert(Box::new(Sign));
        calculations.insert(Box::new(Minimum));
        calculations.insert(Box::new(Maximum));
        calculations.insert(Box::new(Power));
        calculations.insert(Box::new(Remainder));
        calculations.insert(Box::new(Hypotenuse));
        calculations
    }

    fn insert(&mut self, calculation: Box<dyn Calculation>) {
        let name = calculation.name();
        self.entries.insert(name, calculation);
    }

    fn find(&self, name: &Identifier) -> Option<&dyn Calculation> {
        self.entries
            .get(name)
            .map(|calculation| calculation.as_ref())
    }

    fn contains(&self, name: &Identifier) -> bool {
        self.entries.contains_key(name)
    }
}

struct Session<'a> {
    variables: std::collections::HashMap<Identifier, Value>,
    answer: Option<Value>,
    constants: &'a Constants,
    calculations: &'a Calculations,
    angle: Angle,
}

impl<'a> Session<'a> {
    fn new(
        constants: &'a Constants,
        calculations: &'a Calculations,
        angle: Angle,
    ) -> Self {
        use std::collections::HashMap;
        Self {
            variables: HashMap::new(),
            answer: None,
            constants,
            calculations,
            angle,
        }
    }

    fn lookup(&self, name: &Identifier) -> Result<Value, Failure> {
        if name.is_answer() {
            return self
                .answer
                .ok_or_else(|| Failure::new("ans is not defined"));
        }
        if let Some(value) = self.constants.find(name) {
            return Ok(value);
        }
        if let Some(value) = self.variables.get(name) {
            return Ok(*value);
        }
        Err(Failure::new(format!("unknown name {name}")))
    }

    fn assign(&mut self, name: Identifier, value: Value) -> Result<(), Failure> {
        if name.is_answer()
            || self.constants.contains(&name)
            || self.calculations.contains(&name)
        {
            return Err(Failure::new(format!("{name} is reserved")));
        }
        self.variables.insert(name, value);
        Ok(())
    }

    fn remember(&mut self, value: Value) {
        self.answer = Some(value);
    }

    fn calculate(
        &self,
        name: &Identifier,
        arguments: Arguments,
    ) -> Result<Value, Failure> {
        let calculation = self
            .calculations
            .find(name)
            .ok_or_else(|| Failure::new(format!("unknown function {name}")))?;
        calculation.validate(&arguments)?;
        calculation.calculate(&arguments, self.angle)
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
                        let precedence = operator.precedence();
                        if precedence >= minimum {
                            self.tokens.bump();
                            let arguments = Arguments::from(vec![left]);
                            left = operator.apply(&arguments)?;
                            continue;
                        }
                    }
                }
                if let Some(operator) = self.operators.infix(symbol) {
                    let precedence = operator.precedence();
                    let associativity = operator.associativity();
                    if precedence < minimum {
                        break;
                    }
                    self.tokens.bump();
                    let next_minimum = match associativity {
                        Associativity::Right => precedence,
                        _ => precedence.next(),
                    };
                    let right = self.expression(next_minimum)?;
                    let arguments = Arguments::from(vec![left, right]);
                    let operator = self
                        .operators
                        .infix(symbol)
                        .ok_or_else(|| Failure::new("missing operator"))?;
                    left = operator.apply(&arguments)?;
                    continue;
                }
            }
            if token.starts_value() {
                let precedence = match self.operators.multiplication() {
                    Some(operator) => operator.precedence(),
                    None => break,
                };
                if precedence < minimum {
                    break;
                }
                let right = self.expression(precedence.next())?;
                let arguments = Arguments::from(vec![left, right]);
                let operator = self
                    .operators
                    .multiplication()
                    .ok_or_else(|| Failure::new("missing operator"))?;
                left = operator.apply(&arguments)?;
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
            Token::Number(value) => Ok(value),
            Token::Name(name) => self.name(name),
            Token::LeftParenthesis => {
                let value = self.expression(Precedence::LOWEST)?;
                self.expect_right_parenthesis()?;
                Ok(value)
            }
            Token::Symbol(symbol) => {
                let precedence = self
                    .operators
                    .prefix(&symbol)
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {symbol}")))?
                    .precedence();
                let value = self.expression(precedence)?;
                let arguments = Arguments::from(vec![value]);
                self.operators
                    .prefix(&symbol)
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {symbol}")))?
                    .apply(&arguments)
            }
            other => Err(Failure::new(format!("unexpected {other}"))),
        }
    }

    fn name(&mut self, name: Identifier) -> Result<Value, Failure> {
        if matches!(self.tokens.peek(), Some(Token::LeftParenthesis)) {
            self.tokens.bump();
            let arguments = self.arguments()?;
            self.session.calculate(&name, arguments)
        } else {
            self.session.lookup(&name)
        }
    }

    fn arguments(&mut self) -> Result<Arguments, Failure> {
        if matches!(self.tokens.peek(), Some(Token::RightParenthesis)) {
            self.tokens.bump();
            return Ok(Arguments::from(Vec::new()));
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
        Ok(Arguments::from(values))
    }

    fn expect_right_parenthesis(&mut self) -> Result<(), Failure> {
        match self.tokens.bump() {
            Some(Token::RightParenthesis) => Ok(()),
            Some(token) => Err(Failure::new(format!("expected ')', found {token}"))),
            None => Err(Failure::new("expected ')'")),
        }
    }
}

pub(crate) struct Calculator {
    operators: Operators,
    calculations: Calculations,
    constants: Constants,
}

impl Calculator {
    pub(crate) fn new() -> Self {
        Self {
            operators: Operators::new(),
            calculations: Calculations::new(),
            constants: Constants::new(),
        }
    }

    pub(crate) fn evaluate(&self, source: &Source, angle: Angle) -> Results {
        let mut session = Session::new(&self.constants, &self.calculations, angle);
        let mut evaluations = Vec::with_capacity(source.line_count());
        evaluations.extend(
            source
                .lines()
                .map(|line| self.evaluate_line(line, &mut session)),
        );
        Results::from(evaluations)
    }

    fn evaluate_line(&self, line: Line<'_>, session: &mut Session<'_>) -> Evaluation {
        match line.statement() {
            Ok(Statement::Empty) => Evaluation::Blank,
            Ok(Statement::Expression(expression)) => {
                match self.evaluate_expression(&expression, session) {
                    Ok(value) => {
                        session.remember(value);
                        Evaluation::Value(value)
                    }
                    Err(failure) => Evaluation::Failure(failure),
                }
            }
            Ok(Statement::Assignment { name, expression }) => {
                match self.evaluate_expression(&expression, session) {
                    Ok(value) => match session.assign(name, value) {
                        Ok(()) => {
                            session.remember(value);
                            Evaluation::Value(value)
                        }
                        Err(failure) => Evaluation::Failure(failure),
                    },
                    Err(failure) => Evaluation::Failure(failure),
                }
            }
            Err(failure) => Evaluation::Failure(failure),
        }
    }

    fn evaluate_expression(
        &self,
        expression: &Expression,
        session: &mut Session<'_>,
    ) -> Result<Value, Failure> {
        let mut lexer = Lexer::new(expression);
        let tokens = lexer.tokens()?;
        let value = Parser::new(tokens, &self.operators, session).value()?;
        if value.is_nan() {
            return Err(Failure::new("not a real number"));
        }
        Ok(value)
    }
}
