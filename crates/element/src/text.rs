pub struct Text(String);

impl crate::Element for Text {}

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
