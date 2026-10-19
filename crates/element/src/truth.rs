#[derive(Clone, Copy, Debug)]
pub struct Truth(bool);

impl crate::Element for Truth {}

impl Truth {
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    pub fn value(self) -> bool {
        self.0
    }
}
