pub trait Element: std::any::Any {
    fn equals(&self, _other: &dyn std::any::Any) -> Option<bool> {
        None
    }

    fn describe(&self) -> crate::Text {
        crate::Text::new("a value")
    }
}
