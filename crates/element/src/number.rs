#[derive(Clone, Copy, Debug)]
pub struct Number(f64);

impl crate::Element for Number {
    fn equals(&self, other: &dyn std::any::Any) -> Option<bool> {
        other.downcast_ref::<Self>().map(|other| self.0 == other.0)
    }
}

impl Number {
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}
