#[derive(Clone, Debug)]
pub enum Failure {
    Arity {
        name: crate::Text,
        expected: crate::Arity,
    },
    Type {
        expected: crate::Text,
        actual: crate::Text,
    },
    Domain {
        name: crate::Text,
        requirement: crate::Text,
    },
    ZeroDivisor {
        operation: crate::Text,
    },
    UnknownName {
        name: crate::Text,
    },
    Reserved {
        name: crate::Text,
    },
    TooDeep,
    Message(crate::Text),
}

impl Failure {
    pub fn new(message: impl Into<crate::Text>) -> Self {
        Self::Message(message.into())
    }

    fn sentence(&self) -> String {
        match self {
            Self::Arity { name, expected } => format!("{name} expects {expected}"),
            Self::Type { expected, actual } => format!("expected {expected}, received {actual}"),
            Self::Domain { name, requirement } => format!("{name} requires {requirement}"),
            Self::ZeroDivisor { operation } => format!("{operation} by zero"),
            Self::UnknownName { name } => format!("unknown name {name}"),
            Self::Reserved { name } => format!("{name} is reserved"),
            Self::TooDeep => String::from("recursion is too deep"),
            Self::Message(message) => message.as_ref().to_string(),
        }
    }
}

impl crate::Element for Failure {
    fn describe(&self) -> crate::Text {
        crate::Text::new("an error")
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sentence = self.sentence();
        let mut characters = sentence.chars();
        match characters.next() {
            Some(first) => write!(
                formatter,
                "{}{}",
                first.to_uppercase().collect::<String>(),
                characters.as_str()
            ),
            None => Ok(()),
        }
    }
}

impl std::error::Error for Failure {}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
