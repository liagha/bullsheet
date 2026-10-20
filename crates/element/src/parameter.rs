#[derive(Clone, Debug)]
pub struct Parameter {
    name: crate::Text,
}

impl crate::Element for Parameter {
    fn describe(&self) -> crate::Text {
        crate::Text::new("a parameter")
    }
}

impl Parameter {
    pub fn new(name: impl Into<crate::Text>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &crate::Text {
        &self.name
    }
}
