#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arity {
    minimum: usize,
    maximum: Option<usize>,
}

impl Arity {
    pub const fn exactly(count: usize) -> Self {
        Self {
            minimum: count,
            maximum: Some(count),
        }
    }

    pub const fn between(minimum: usize, maximum: usize) -> Self {
        Self {
            minimum,
            maximum: Some(maximum),
        }
    }

    pub const fn at_least(minimum: usize) -> Self {
        Self {
            minimum,
            maximum: None,
        }
    }

    pub fn accepts(self, count: usize) -> bool {
        count >= self.minimum && self.maximum.is_none_or(|maximum| count <= maximum)
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

pub trait Callable: 'static {
    fn name(&self) -> crate::Text;

    fn arity(&self) -> Arity;

    fn lazy(&self) -> bool {
        false
    }

    fn call(&self, arguments: &[crate::Value]) -> Result<crate::Value, crate::Failure>;

    fn check(&self, arguments: &[crate::Value]) -> Result<(), crate::Failure> {
        let arity = self.arity();
        if arity.accepts(arguments.len()) {
            Ok(())
        } else {
            Err(crate::Failure::Arity {
                name: self.name(),
                expected: arity,
            })
        }
    }
}
