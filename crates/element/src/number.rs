#[derive(Clone, Copy, Debug)]
pub struct Number(f64);

impl crate::Element for Number {}

impl Number {
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}
