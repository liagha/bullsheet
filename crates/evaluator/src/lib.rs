mod lexer;
mod parser;
mod syntax;

pub use crate::syntax::Source;

use crate::lexer::{Glyph, Lexer};
use crate::parser::{Associativity, Fixity, Grammar, Parser, Precedence, Shape, ShapeKind};
use crate::syntax::{Expression, Identifier, Line, Span, Statement};
use element::{Blank, Callable, Failure, Function, Key, Number, Text, Truth, Value};

pub struct Part {
    span: Span,
    value: Value,
}

impl Part {
    pub fn span(&self) -> Span {
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
        values.len() >= self.minimum && self.maximum.is_none_or(|maximum| values.len() <= maximum)
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
            Some(maximum) => write!(formatter, "{} or {}", self.minimum, Self::counted(maximum)),
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
        if let (Some(left), Some(right)) = (values[0].cast::<Key>(), values[1].cast::<Key>()) {
            return Ok(Value::new(left.combine(right)?));
        }
        if let (Some(key), Some(number)) = (values[0].cast::<Key>(), values[1].cast::<Number>()) {
            return Self::digit(key, number);
        }
        if let (Some(number), Some(key)) = (values[0].cast::<Number>(), values[1].cast::<Key>()) {
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
        Ok(Value::new(Number::new(
            arguments.number(0)?.value() / 100.0,
        )))
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

const COMPARISONS: [(Glyph, &str, fn(f64, f64) -> bool); 4] = [
    (Glyph::LESS, "less", |left, right| left < right),
    (Glyph::GREATER, "greater", |left, right| left > right),
    (Glyph::LESS_OR_EQUAL, "less_or_equal", |left, right| {
        left <= right
    }),
    (
        Glyph::GREATER_OR_EQUAL,
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
            return Err(Failure::new("square root requires a non-negative number"));
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
                return Err(Failure::new("logarithm base must be positive and not one"));
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
        Ok(Value::new(Number::new(
            arguments.number(0)?.value().floor(),
        )))
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
        Ok(Value::new(Number::new(
            arguments.number(0)?.value().trunc(),
        )))
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
        Ok(Value::new(Number::new(left.value().hypot(right.value()))))
    }
}

struct Operators {
    prefixes: std::collections::HashMap<Glyph, Operator>,
    infixes: std::collections::HashMap<Glyph, Operator>,
    postfixes: std::collections::HashMap<Glyph, Operator>,
}

impl Operators {
    fn new() -> Self {
        use std::collections::HashMap;
        let mut prefixes = HashMap::new();
        prefixes.insert(
            Glyph::PLUS,
            Operator::new(Positive, Precedence::PREFIX, Associativity::None),
        );
        prefixes.insert(
            Glyph::MINUS,
            Operator::new(Negation, Precedence::PREFIX, Associativity::None),
        );
        let mut infixes = HashMap::new();
        infixes.insert(
            Glyph::PLUS,
            Operator::new(Addition, Precedence::ADDITION, Associativity::Left),
        );
        infixes.insert(
            Glyph::MINUS,
            Operator::new(Subtraction, Precedence::ADDITION, Associativity::Left),
        );
        infixes.insert(
            Glyph::STAR,
            Operator::new(
                Multiplication,
                Precedence::MULTIPLICATION,
                Associativity::Left,
            ),
        );
        infixes.insert(
            Glyph::SLASH,
            Operator::new(Division, Precedence::MULTIPLICATION, Associativity::Left),
        );
        infixes.insert(
            Glyph::PERCENT,
            Operator::new(Modulo, Precedence::MULTIPLICATION, Associativity::Left),
        );
        infixes.insert(
            Glyph::CARET,
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
            Glyph::EQUAL,
            Operator::new(Equality, Precedence::COMPARISON, Associativity::Left),
        );
        infixes.insert(
            Glyph::NOT_EQUAL,
            Operator::new(Inequality, Precedence::COMPARISON, Associativity::Left),
        );
        let mut postfixes = HashMap::new();
        postfixes.insert(
            Glyph::BANG,
            Operator::new(Factorial, Precedence::POSTFIX, Associativity::None),
        );
        postfixes.insert(
            Glyph::PERCENT,
            Operator::new(Percentage, Precedence::POSTFIX, Associativity::None),
        );
        Self {
            prefixes,
            infixes,
            postfixes,
        }
    }

    fn prefix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.prefixes.get(symbol)
    }

    fn infix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.infixes.get(symbol)
    }

    fn postfix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.postfixes.get(symbol)
    }

    fn value(&self, symbol: &Glyph) -> Option<Value> {
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
        entries.insert(
            Identifier::new("not"),
            Value::new(Function::new(LogicalNot)),
        );
        entries.insert(
            Identifier::new("and"),
            Value::new(Function::new(LogicalAnd)),
        );
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
        entries.insert(Identifier::new("sin"), Value::new(Function::new(Sine)));
        entries.insert(Identifier::new("cos"), Value::new(Function::new(Cosine)));
        entries.insert(Identifier::new("tan"), Value::new(Function::new(Tangent)));
        entries.insert(Identifier::new("asin"), Value::new(Function::new(Arcsine)));
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
        entries.insert(Identifier::new("cbrt"), Value::new(Function::new(CubeRoot)));
        entries.insert(Identifier::new("abs"), Value::new(Function::new(Absolute)));
        entries.insert(
            Identifier::new("ln"),
            Value::new(Function::new(NaturalLogarithm)),
        );
        entries.insert(Identifier::new("log"), Value::new(Function::new(Logarithm)));
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
        entries.insert(Identifier::new("ceil"), Value::new(Function::new(Ceiling)));
        entries.insert(Identifier::new("round"), Value::new(Function::new(Round)));
        entries.insert(
            Identifier::new("trunc"),
            Value::new(Function::new(Truncate)),
        );
        entries.insert(Identifier::new("sign"), Value::new(Function::new(Sign)));
        entries.insert(Identifier::new("min"), Value::new(Function::new(Minimum)));
        entries.insert(Identifier::new("max"), Value::new(Function::new(Maximum)));
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

struct Symbol {
    name: Identifier,
    parameters: Vec<Identifier>,
    body: Shape,
    scope: std::rc::Weak<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Symbol {
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
        Evaluator::new(&session).value(&self.body)
    }
}

impl Callable for Symbol {
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

struct Thunk {
    shape: std::rc::Rc<Shape>,
    variables: std::rc::Rc<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Callable for Thunk {
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
        Evaluator::new(&session).value(&self.shape)
    }
}

impl Grammar for Operators {
    fn fixity(&self, glyph: Glyph) -> Fixity {
        Fixity::new(
            self.prefixes
                .get(&glyph)
                .map(|operator| operator.precedence),
            self.infixes
                .get(&glyph)
                .map(|operator| (operator.precedence, operator.associativity)),
            self.postfixes
                .get(&glyph)
                .map(|operator| operator.precedence),
        )
    }
}

struct Evaluator<'a> {
    session: &'a Session,
    parts: Option<&'a mut Vec<Part>>,
}

impl<'a> Evaluator<'a> {
    fn new(session: &'a Session) -> Self {
        Self {
            session,
            parts: None,
        }
    }

    fn build(session: &'a Session, parts: &'a mut Vec<Part>) -> Self {
        Self {
            session,
            parts: Some(parts),
        }
    }

    fn value(&mut self, shape: &Shape) -> Result<Value, Failure> {
        let result = self.evaluate(shape);
        if Self::is_leaf(shape) {
            let recorded = match &result {
                Ok(value) => Some(value.clone()),
                Err(failure) if matches!(shape.kind(), ShapeKind::Name(_)) => {
                    Some(Value::new(Failure::new(failure.to_string())))
                }
                Err(_) => None,
            };
            if let Some(value) = recorded {
                self.record(shape, value);
            }
        }
        result
    }

    fn is_leaf(shape: &Shape) -> bool {
        matches!(
            shape.kind(),
            ShapeKind::Number(_) | ShapeKind::Key(_) | ShapeKind::Name(_) | ShapeKind::Glyph(_)
        )
    }

    fn evaluate(&mut self, shape: &Shape) -> Result<Value, Failure> {
        match shape.kind() {
            ShapeKind::Number(number) => Ok(Value::new(*number)),
            ShapeKind::Key(key) => Ok(Value::new(key.clone())),
            ShapeKind::Name(name) => self.session.lookup(name),
            ShapeKind::Glyph(glyph) => self
                .session
                .language
                .operators
                .value(glyph)
                .ok_or_else(|| Failure::new(format!("unexpected symbol {glyph}"))),
            ShapeKind::Unary { operator, operand } => {
                let function = self.operator(operator, Operators::prefix)?;
                let value = self.value(operand)?;
                function.call(&[value])
            }
            ShapeKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.value(left)?;
                let function = self.operator(operator, Operators::infix)?;
                let right = self.value(right)?;
                function.call(&[left, right])
            }
            ShapeKind::Postfix { operator, operand } => {
                let value = self.value(operand)?;
                let function = self.operator(operator, Operators::postfix)?;
                function.call(&[value])
            }
            ShapeKind::Multiplication {
                operator,
                left,
                right,
            } => {
                let function = self
                    .session
                    .language
                    .operators
                    .infix(operator)
                    .map(|operator| operator.function.clone())
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {operator}")))?;
                let left = self.value(left)?;
                let right = self.value(right)?;
                function.call(&[left, right])
            }
            ShapeKind::Call {
                function,
                arguments,
            } => {
                let name = match function.kind() {
                    ShapeKind::Name(name) => name.to_string(),
                    _ => String::from("expression"),
                };
                let value = self.value(function)?;
                let callable = value
                    .cast::<Function>()
                    .cloned()
                    .ok_or_else(|| Failure::new(format!("{name} is not a function")))?;
                let values = if callable.lazy() {
                    arguments
                        .iter()
                        .map(|argument| Value::new(Function::new(self.thunk(argument))))
                        .collect()
                } else {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(self.value(argument)?);
                    }
                    values
                };
                callable.call(&values)
            }
        }
    }

    fn operator(
        &mut self,
        shape: &Shape,
        select: for<'b> fn(&'b Operators, &Glyph) -> Option<&'b Operator>,
    ) -> Result<Function, Failure> {
        let glyph = match shape.kind() {
            ShapeKind::Glyph(glyph) => glyph,
            _ => return Err(Failure::new("unexpected operator")),
        };
        let function = select(&self.session.language.operators, &glyph)
            .map(|operator| operator.function.clone())
            .ok_or_else(|| Failure::new(format!("unexpected symbol {glyph}")))?;
        self.record(shape, Value::new(function.clone()));
        Ok(function)
    }

    fn record(&mut self, shape: &Shape, value: Value) {
        if let Some(parts) = self.parts.as_deref_mut() {
            parts.push(Part {
                span: shape.span(),
                value,
            });
        }
    }

    fn thunk(&self, shape: &Shape) -> Thunk {
        Thunk {
            shape: std::rc::Rc::new(shape.clone()),
            variables: std::rc::Rc::clone(&self.session.variables),
            language: std::rc::Rc::clone(&self.session.language),
            depth: std::rc::Rc::clone(&self.session.depth),
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
        readings.extend(
            source
                .lines()
                .map(|line| self.evaluate_line(line, &session)),
        );
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
            Ok(Statement::Assignment {
                name,
                span,
                expression,
            }) => {
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
                let body = match self.parse(&expression, session) {
                    Ok(body) => body,
                    Err(failure) => return Reading::without_parts(Value::new(failure)),
                };
                let mut names = Vec::with_capacity(parameters.len());
                let mut parts = Vec::with_capacity(parameters.len() + 1);
                for (parameter, span) in parameters {
                    names.push(parameter.clone());
                    parts.push(Part {
                        span,
                        value: Value::new(Parameter::new(&parameter)),
                    });
                }
                let function = Value::new(Function::new(Symbol {
                    name: name.clone(),
                    parameters: names,
                    body,
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

    fn parse(&self, expression: &Expression, session: &Session) -> Result<Shape, Failure> {
        let mut lexer = Lexer::new(expression);
        let tokens = lexer.tokens()?;
        Parser::new(tokens, &session.language.operators, expression).shape()
    }

    fn evaluate_expression(&self, expression: &Expression, session: &Session) -> Reading {
        let mut parts = Vec::new();
        let result = self
            .parse(expression, session)
            .and_then(|shape| Evaluator::build(session, &mut parts).value(&shape));
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
