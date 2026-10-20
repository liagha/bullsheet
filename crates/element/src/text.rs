#[derive(Clone, Debug)]
pub struct Text(String);

impl crate::Element for Text {
    fn equals(&self, other: &dyn std::any::Any) -> Option<bool> {
        other
            .downcast_ref::<Self>()
            .map(|other| self.0 == other.0)
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
