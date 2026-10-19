#[derive(Clone)]
pub struct Value(std::rc::Rc<dyn crate::Element>);

impl Value {
    pub fn new(element: impl crate::Element) -> Self {
        Self(std::rc::Rc::new(element))
    }

    pub fn any(&self) -> &dyn std::any::Any {
        self.0.as_ref()
    }

    pub fn cast<T: crate::Element>(&self) -> Option<&T> {
        self.any().downcast_ref()
    }
}
