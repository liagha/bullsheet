#[derive(Debug)]
pub struct Presentation(String);

impl Presentation {
    pub fn new(text: impl Into<String>) -> Self {
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
pub struct Failure(Presentation);

impl Failure {
    pub fn new(message: impl Into<String>) -> Self {
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Value(f64);

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
    pub fn plain(&self) -> Presentation {
        Presentation::new(self.formatted(None))
    }

    pub fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    pub fn subtract(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }

    pub fn multiply(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    pub fn divide(self, other: Self) -> Result<Self, Failure> {
        if other.is_zero() {
            Err(Failure::new("division by zero"))
        } else {
            Ok(Self(self.0 / other.0))
        }
    }

    pub fn remainder(self, other: Self) -> Result<Self, Failure> {
        if other.is_zero() {
            Err(Failure::new("remainder by zero"))
        } else {
            Ok(Self(self.0 % other.0))
        }
    }

    pub fn power(self, other: Self) -> Self {
        Self(self.0.powf(other.0))
    }

    pub fn negate(self) -> Self {
        Self(-self.0)
    }

    pub fn percentage(self) -> Self {
        Self(self.0 / 100.0)
    }

    pub fn factorial(self) -> Result<Self, Failure> {
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

    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0.0
    }

    pub fn is_nan(&self) -> bool {
        self.0.is_nan()
    }

    pub fn is_infinite(&self) -> bool {
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

impl Value {
    pub fn absolute(self) -> Self {
        Self(self.0.abs())
    }

    pub fn square_root(self) -> Result<Self, Failure> {
        if self.is_negative() {
            Err(Failure::new("square root requires a non-negative number"))
        } else {
            Ok(Self(self.0.sqrt()))
        }
    }

    pub fn cube_root(self) -> Self {
        Self(self.0.cbrt())
    }

    pub fn natural_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.ln()))
        }
    }

    pub fn binary_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.log2()))
        }
    }

    pub fn decimal_logarithm(self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            Err(Failure::new("logarithm requires a positive number"))
        } else {
            Ok(Self(self.0.log10()))
        }
    }

    pub fn logarithm(self, base: Self) -> Result<Self, Failure> {
        if self.0 <= 0.0 {
            return Err(Failure::new("logarithm requires a positive number"));
        }
        if base.0 <= 0.0 || base.0 == 1.0 {
            return Err(Failure::new("logarithm base must be positive and not one"));
        }
        Ok(Self(self.0.log(base.0)))
    }

    pub fn exponential(self) -> Self {
        Self(self.0.exp())
    }

    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    pub fn ceiling(self) -> Self {
        Self(self.0.ceil())
    }

    pub fn round(self) -> Self {
        Self(self.0.round())
    }

    pub fn round_to(self, places: Self) -> Result<Self, Failure> {
        if places.0 < 0.0 || places.0.fract() != 0.0 || places.0 > 15.0 {
            return Err(Failure::new(
                "round requires a whole number of places between zero and fifteen",
            ));
        }
        let factor = 10.0_f64.powi(places.0 as i32);
        Ok(Self((self.0 * factor).round() / factor))
    }

    pub fn truncate(self) -> Self {
        Self(self.0.trunc())
    }

    pub fn sign(self) -> Self {
        if self.is_zero() {
            Self(0.0)
        } else {
            Self(self.0.signum())
        }
    }

    pub fn sine(self) -> Self {
        Self(self.0.sin())
    }

    pub fn cosine(self) -> Self {
        Self(self.0.cos())
    }

    pub fn tangent(self) -> Self {
        Self(self.0.tan())
    }

    pub fn arcsine(self) -> Result<Self, Failure> {
        if !(-1.0..=1.0).contains(&self.0) {
            Err(Failure::new("arcsine requires a number between minus one and one"))
        } else {
            Ok(Self(self.0.asin()))
        }
    }

    pub fn arccosine(self) -> Result<Self, Failure> {
        if !(-1.0..=1.0).contains(&self.0) {
            Err(Failure::new("arccosine requires a number between minus one and one"))
        } else {
            Ok(Self(self.0.acos()))
        }
    }

    pub fn arctangent(self) -> Self {
        Self(self.0.atan())
    }

    pub fn arctangent2(self, other: Self) -> Self {
        Self(self.0.atan2(other.0))
    }

    pub fn hyperbolic_sine(self) -> Self {
        Self(self.0.sinh())
    }

    pub fn hyperbolic_cosine(self) -> Self {
        Self(self.0.cosh())
    }

    pub fn hyperbolic_tangent(self) -> Self {
        Self(self.0.tanh())
    }

    pub fn minimum(values: &[Self]) -> Result<Self, Failure> {
        values
            .iter()
            .copied()
            .reduce(|left, right| Self(left.0.min(right.0)))
            .ok_or_else(|| Failure::new("minimum accepts at least one argument"))
    }

    pub fn maximum(values: &[Self]) -> Result<Self, Failure> {
        values
            .iter()
            .copied()
            .reduce(|left, right| Self(left.0.max(right.0)))
            .ok_or_else(|| Failure::new("maximum accepts at least one argument"))
    }

    pub fn hypotenuse(self, other: Self) -> Self {
        Self(self.0.hypot(other.0))
    }
}
