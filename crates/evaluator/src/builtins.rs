use crate::lexer::Glyph;
use crate::parser::{Associativity, Fixity, Grammar, Precedence};
use crate::registry::Registry;
use element::{Arity, Arguments, Callable, Failure, Function, Key, Number, Text, Truth, Value};

pub struct Operator {
    function: Function,
    precedence: Precedence,
    associativity: Associativity,
}

impl Operator {
    fn new(function: Function, precedence: Precedence, associativity: Associativity) -> Self {
        Self {
            function,
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(arguments.number(0)?))
    }
}

struct Negation;

impl Callable for Negation {
    fn name(&self) -> Text {
        Text::new("negate")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(-arguments.number(0)?.value())))
    }
}

struct Addition;

impl Callable for Addition {
    fn name(&self) -> Text {
        Text::new("add")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() - right.value())))
    }
}

struct Multiplication;

impl Callable for Multiplication {
    fn name(&self) -> Text {
        Text::new("multiply")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value() * right.value())))
    }
}

struct Division;

impl Callable for Division {
    fn name(&self) -> Text {
        Text::new("divide")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        if right.value() == 0.0 {
            return Err(Failure::ZeroDivisor {
                operation: Text::new("division"),
            });
        }
        Ok(Value::new(Number::new(left.value() / right.value())))
    }
}

struct Modulo;

impl Callable for Modulo {
    fn name(&self) -> Text {
        Text::new("mod")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        if right.value() == 0.0 {
            return Err(Failure::ZeroDivisor {
                operation: Text::new("remainder"),
            });
        }
        Ok(Value::new(Number::new(left.value() % right.value())))
    }
}

struct Power;

impl Callable for Power {
    fn name(&self) -> Text {
        Text::new("pow")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value().powf(right.value()))))
    }
}

struct Factorial;

impl Callable for Factorial {
    fn name(&self) -> Text {
        Text::new("factorial")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value < 0.0 || value.fract() != 0.0 {
            return Err(Failure::Domain {
                name: Text::new("factorial"),
                requirement: Text::new("a non-negative integer"),
            });
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Truth::new(!arguments.truth(0)?)))
    }
}

struct LogicalAnd;

impl Callable for LogicalAnd {
    fn name(&self) -> Text {
        Text::new("and")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Truth::new(
            arguments.truth(0)? || arguments.truth(1)?,
        )))
    }
}

struct Comparison {
    name: &'static str,
    compare: fn(f64, f64) -> bool,
}

impl Callable for Comparison {
    fn name(&self) -> Text {
        Text::new(self.name)
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
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

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
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

    fn arity(&self) -> Arity {
        Arity::exactly(3)
    }

    fn lazy(&self) -> bool {
        true
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().sin())))
    }
}

struct Cosine;

impl Callable for Cosine {
    fn name(&self) -> Text {
        Text::new("cos")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().cos())))
    }
}

struct Tangent;

impl Callable for Tangent {
    fn name(&self) -> Text {
        Text::new("tan")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().tan())))
    }
}

struct Arcsine;

impl Callable for Arcsine {
    fn name(&self) -> Text {
        Text::new("asin")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::Domain {
                name: Text::new("asin"),
                requirement: Text::new("a number between minus one and one"),
            });
        }
        Ok(Value::new(Number::new(value.asin())))
    }
}

struct Arccosine;

impl Callable for Arccosine {
    fn name(&self) -> Text {
        Text::new("acos")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if !(-1.0..=1.0).contains(&value) {
            return Err(Failure::Domain {
                name: Text::new("acos"),
                requirement: Text::new("a number between minus one and one"),
            });
        }
        Ok(Value::new(Number::new(value.acos())))
    }
}

struct Arctangent;

impl Callable for Arctangent {
    fn name(&self) -> Text {
        Text::new("atan")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        Ok(Value::new(Number::new(value.atan())))
    }
}

struct Arctangent2;

impl Callable for Arctangent2 {
    fn name(&self) -> Text {
        Text::new("atan2")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value().atan2(right.value()))))
    }
}

struct HyperbolicSine;

impl Callable for HyperbolicSine {
    fn name(&self) -> Text {
        Text::new("sinh")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().sinh())))
    }
}

struct HyperbolicCosine;

impl Callable for HyperbolicCosine {
    fn name(&self) -> Text {
        Text::new("cosh")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().cosh())))
    }
}

struct HyperbolicTangent;

impl Callable for HyperbolicTangent {
    fn name(&self) -> Text {
        Text::new("tanh")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().tanh())))
    }
}

struct SquareRoot;

impl Callable for SquareRoot {
    fn name(&self) -> Text {
        Text::new("sqrt")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value < 0.0 {
            return Err(Failure::Domain {
                name: Text::new("sqrt"),
                requirement: Text::new("a non-negative number"),
            });
        }
        Ok(Value::new(Number::new(value.sqrt())))
    }
}

struct CubeRoot;

impl Callable for CubeRoot {
    fn name(&self) -> Text {
        Text::new("cbrt")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().cbrt())))
    }
}

struct Absolute;

impl Callable for Absolute {
    fn name(&self) -> Text {
        Text::new("abs")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().abs())))
    }
}

struct NaturalLogarithm;

impl Callable for NaturalLogarithm {
    fn name(&self) -> Text {
        Text::new("ln")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::Domain {
                name: Text::new("ln"),
                requirement: Text::new("a positive number"),
            });
        }
        Ok(Value::new(Number::new(value.ln())))
    }
}

struct Logarithm;

impl Callable for Logarithm {
    fn name(&self) -> Text {
        Text::new("log")
    }

    fn arity(&self) -> Arity {
        Arity::between(1, 2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::Domain {
                name: Text::new("log"),
                requirement: Text::new("a positive number"),
            });
        }
        let result = if arguments.len() == 1 {
            value.log10()
        } else {
            let base = arguments.number(1)?.value();
            if base <= 0.0 || base == 1.0 {
                return Err(Failure::Domain {
                    name: Text::new("log"),
                    requirement: Text::new("a base that is positive and not one"),
                });
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::Domain {
                name: Text::new("log2"),
                requirement: Text::new("a positive number"),
            });
        }
        Ok(Value::new(Number::new(value.log2())))
    }
}

struct DecimalLogarithm;

impl Callable for DecimalLogarithm {
    fn name(&self) -> Text {
        Text::new("log10")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if value <= 0.0 {
            return Err(Failure::Domain {
                name: Text::new("log10"),
                requirement: Text::new("a positive number"),
            });
        }
        Ok(Value::new(Number::new(value.log10())))
    }
}

struct Exponential;

impl Callable for Exponential {
    fn name(&self) -> Text {
        Text::new("exp")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().exp())))
    }
}

struct Floor;

impl Callable for Floor {
    fn name(&self) -> Text {
        Text::new("floor")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        Ok(Value::new(Number::new(arguments.number(0)?.value().ceil())))
    }
}

struct Round;

impl Callable for Round {
    fn name(&self) -> Text {
        Text::new("round")
    }

    fn arity(&self) -> Arity {
        Arity::between(1, 2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let value = arguments.number(0)?.value();
        if arguments.len() == 1 {
            return Ok(Value::new(Number::new(value.round())));
        }
        let places = arguments.number(1)?.value();
        if places < 0.0 || places.fract() != 0.0 || places > 15.0 {
            return Err(Failure::Domain {
                name: Text::new("round"),
                requirement: Text::new("a whole number of places between zero and fifteen"),
            });
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::at_least(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::at_least(1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
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

    fn arity(&self) -> Arity {
        Arity::exactly(2)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let (left, right) = (arguments.number(0)?, arguments.number(1)?);
        Ok(Value::new(Number::new(left.value().hypot(right.value()))))
    }
}

pub struct Operators {
    prefixes: std::collections::HashMap<Glyph, Operator>,
    infixes: std::collections::HashMap<Glyph, Operator>,
    postfixes: std::collections::HashMap<Glyph, Operator>,
}

const PREFIXES: [(Glyph, &str, Precedence); 2] = [
    (Glyph::PLUS, "positive", Precedence::PREFIX),
    (Glyph::MINUS, "negate", Precedence::PREFIX),
];

const INFIXES: [(Glyph, &str, Precedence, Associativity); 8] = [
    (Glyph::PLUS, "add", Precedence::ADDITION, Associativity::Left),
    (Glyph::MINUS, "subtract", Precedence::ADDITION, Associativity::Left),
    (
        Glyph::STAR,
        "multiply",
        Precedence::MULTIPLICATION,
        Associativity::Left,
    ),
    (
        Glyph::SLASH,
        "divide",
        Precedence::MULTIPLICATION,
        Associativity::Left,
    ),
    (
        Glyph::PERCENT,
        "mod",
        Precedence::MULTIPLICATION,
        Associativity::Left,
    ),
    (Glyph::CARET, "pow", Precedence::POWER, Associativity::Right),
    (Glyph::EQUAL, "equal", Precedence::COMPARISON, Associativity::Left),
    (
        Glyph::NOT_EQUAL,
        "not_equal",
        Precedence::COMPARISON,
        Associativity::Left,
    ),
];

const COMPARISONS: [(Glyph, &str, Precedence, Associativity, fn(f64, f64) -> bool); 4] = [
    (
        Glyph::LESS,
        "less",
        Precedence::COMPARISON,
        Associativity::Left,
        |left, right| left < right,
    ),
    (
        Glyph::GREATER,
        "greater",
        Precedence::COMPARISON,
        Associativity::Left,
        |left, right| left > right,
    ),
    (
        Glyph::LESS_OR_EQUAL,
        "less_or_equal",
        Precedence::COMPARISON,
        Associativity::Left,
        |left, right| left <= right,
    ),
    (
        Glyph::GREATER_OR_EQUAL,
        "greater_or_equal",
        Precedence::COMPARISON,
        Associativity::Left,
        |left, right| left >= right,
    ),
];

const POSTFIXES: [(Glyph, &str, Precedence); 2] = [
    (Glyph::BANG, "factorial", Precedence::POSTFIX),
    (Glyph::PERCENT, "percentage", Precedence::POSTFIX),
];

impl Operators {
    pub fn new(registry: &Registry) -> Self {
        let mut prefixes = std::collections::HashMap::new();
        for (glyph, name, precedence) in PREFIXES {
            prefixes.insert(
                glyph,
                Operator::new(registry.function(name), precedence, Associativity::None),
            );
        }
        let mut infixes = std::collections::HashMap::new();
        for (glyph, name, precedence, associativity) in INFIXES {
            infixes.insert(
                glyph,
                Operator::new(registry.function(name), precedence, associativity),
            );
        }
        for (glyph, name, precedence, associativity, _) in COMPARISONS {
            infixes.insert(
                glyph,
                Operator::new(registry.function(name), precedence, associativity),
            );
        }
        let mut postfixes = std::collections::HashMap::new();
        for (glyph, name, precedence) in POSTFIXES {
            postfixes.insert(
                glyph,
                Operator::new(registry.function(name), precedence, Associativity::None),
            );
        }
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

impl Registry {
    pub fn add_builtins(&mut self) {
        for (name, number) in [
            ("pi", std::f64::consts::PI),
            ("tau", std::f64::consts::TAU),
            ("e", std::f64::consts::E),
            ("deg", std::f64::consts::PI / 180.0),
            ("phi", (1.0 + 5.0_f64.sqrt()) / 2.0),
            ("inf", f64::INFINITY),
        ] {
            self.add(name, Value::new(Number::new(number)));
        }
        self.add("true", Value::new(Truth::new(true)));
        self.add("false", Value::new(Truth::new(false)));
        self.add_callable("positive", Positive);
        self.add_callable("negate", Negation);
        self.add_callable("add", Addition);
        self.add_callable("subtract", Subtraction);
        self.add_callable("multiply", Multiplication);
        self.add_callable("divide", Division);
        self.add_callable("mod", Modulo);
        self.add_callable("pow", Power);
        self.add_callable("factorial", Factorial);
        self.add_callable("percentage", Percentage);
        self.add_callable("not", LogicalNot);
        self.add_callable("and", LogicalAnd);
        self.add_callable("or", LogicalOr);
        self.add_callable("equal", Equality);
        self.add_callable("not_equal", Inequality);
        self.add_callable("if", Conditional);
        for (_, name, _, _, compare) in COMPARISONS {
            self.add_callable(name, Comparison { name, compare });
        }
        self.add_callable("sin", Sine);
        self.add_callable("cos", Cosine);
        self.add_callable("tan", Tangent);
        self.add_callable("asin", Arcsine);
        self.add_callable("acos", Arccosine);
        self.add_callable("atan", Arctangent);
        self.add_callable("atan2", Arctangent2);
        self.add_callable("sinh", HyperbolicSine);
        self.add_callable("cosh", HyperbolicCosine);
        self.add_callable("tanh", HyperbolicTangent);
        self.add_callable("sqrt", SquareRoot);
        self.add_callable("cbrt", CubeRoot);
        self.add_callable("abs", Absolute);
        self.add_callable("ln", NaturalLogarithm);
        self.add_callable("log", Logarithm);
        self.add_callable("log2", BinaryLogarithm);
        self.add_callable("log10", DecimalLogarithm);
        self.add_callable("exp", Exponential);
        self.add_callable("floor", Floor);
        self.add_callable("ceil", Ceiling);
        self.add_callable("round", Round);
        self.add_callable("trunc", Truncate);
        self.add_callable("sign", Sign);
        self.add_callable("min", Minimum);
        self.add_callable("max", Maximum);
        self.add_callable("hypot", Hypotenuse);
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
