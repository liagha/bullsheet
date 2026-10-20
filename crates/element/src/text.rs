#[derive(Clone, Debug)]
pub struct Text(String);

impl crate::Element for Text {
    fn equals(&self, other: &dyn std::any::Any) -> Option<bool> {
        other
            .downcast_ref::<Self>()
            .map(|other| self.0 == other.0)
    }

    fn describe(&self) -> crate::Text {
        crate::Text::new("a text")
    }
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}
