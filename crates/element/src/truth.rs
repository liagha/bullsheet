#[derive(Clone, Copy, Debug)]
pub struct Truth(bool);

impl crate::Element for Truth {
    fn equals(&self, other: &dyn std::any::Any) -> Option<bool> {
        other.downcast_ref::<Self>().map(|other| self.0 == other.0)
    }

    fn describe(&self) -> crate::Text {
        crate::Text::new("a true or false")
    }
}

impl Truth {
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    pub fn value(self) -> bool {
        self.0
    }
}
