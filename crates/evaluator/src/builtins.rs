use crate::lexer::Glyph;
use crate::parser::{Associativity, Fixity, Grammar, Precedence};
use crate::{Arguments, Arity, Identifier, Name};
use element::{Callable, Failure, Function, Key, Number, Text, Truth, Value};

pub struct Operator {
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

    pub fn function(&self) -> Function {
        self.function.clone()
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
        if let (Some(left), Some(right)) = (values[0].cast::<Text>(), values[1].cast::<Text>()) {
            return Ok(Value::new(Text::new(format!("{left}{right}"))));
        }
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

pub struct Operators {
    prefixes: std::collections::HashMap<Glyph, Operator>,
    infixes: std::collections::HashMap<Glyph, Operator>,
    postfixes: std::collections::HashMap<Glyph, Operator>,
}

impl Operators {
    pub fn new() -> Self {
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

    pub fn prefix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.prefixes.get(symbol)
    }

    pub fn infix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.infixes.get(symbol)
    }

    pub fn postfix(&self, symbol: &Glyph) -> Option<&Operator> {
        self.postfixes.get(symbol)
    }

    pub fn value(&self, symbol: &Glyph) -> Option<Value> {
        self.infix(symbol)
            .or_else(|| self.prefix(symbol))
            .or_else(|| self.postfix(symbol))
            .map(|operator| Value::new(operator.function.clone()))
    }
}

pub struct Constants {
    values: std::collections::HashMap<Identifier, Value>,
}

impl Constants {
    pub fn new() -> Self {
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

    pub fn find(&self, name: &Identifier) -> Option<Value> {
        self.values.get(name).cloned()
    }

    pub fn contains(&self, name: &Identifier) -> bool {
        self.values.contains_key(name)
    }

    pub fn names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = self
            .values
            .iter()
            .map(|(identifier, value)| Name::new(Text::new(identifier.to_string()), value.clone()))
            .collect();
        names.sort_by(|left, right| left.text().as_ref().cmp(right.text().as_ref()));
        names
    }
}

pub struct Functions {
    entries: std::collections::HashMap<Identifier, Value>,
}

impl Functions {
    pub fn new() -> Self {
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

    pub fn find(&self, name: &Identifier) -> Option<Value> {
        self.entries.get(name).cloned()
    }

    pub fn contains(&self, name: &Identifier) -> bool {
        self.entries.contains_key(name)
    }

    pub fn names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = self
            .entries
            .iter()
            .map(|(identifier, value)| Name::new(Text::new(identifier.to_string()), value.clone()))
            .collect();
        names.sort_by(|left, right| left.text().as_ref().cmp(right.text().as_ref()));
        names
    }

    pub fn learn(&mut self, name: Identifier, value: Value) {
        self.entries.insert(name, value);
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
