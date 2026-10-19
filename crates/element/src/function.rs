#[derive(Clone)]
pub struct Function(std::rc::Rc<dyn crate::Callable>);

impl crate::Element for Function {}

impl Function {
    pub fn new(callable: impl crate::Callable) -> Self {
        Self(std::rc::Rc::new(callable))
    }

    pub fn name(&self) -> crate::Text {
        self.0.name()
    }

    pub fn call(&self, arguments: &[crate::Value]) -> Result<crate::Value, crate::Failure> {
        self.0.call(arguments)
    }
}
