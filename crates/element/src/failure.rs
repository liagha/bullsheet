#[derive(Debug)]
pub struct Failure(String);

impl crate::Element for Failure {}

impl Failure {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let mut characters = message.chars();
        let capitalized = match characters.next() {
            Some(first) => first.to_uppercase().chain(characters).collect::<String>(),
            None => String::new(),
        };
        Self(capitalized)
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}
