pub struct Blank;

impl crate::Element for Blank {}

impl Blank {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Blank {
    fn default() -> Self {
        Self::new()
    }
}
