pub trait Element: std::any::Any {
    fn equals(&self, _other: &dyn std::any::Any) -> Option<bool> {
        None
    }
}
