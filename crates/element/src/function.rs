#[derive(Clone)]
pub struct Function(std::rc::Rc<dyn crate::Callable>);

impl crate::Element for Function {
    fn describe(&self) -> crate::Text {
        crate::Text::new("a function")
    }
}

impl Function {
    pub fn new(callable: impl crate::Callable) -> Self {
        Self(std::rc::Rc::new(callable))
    }

    pub fn name(&self) -> crate::Text {
        self.0.name()
    }

    pub fn arity(&self) -> crate::Arity {
        self.0.arity()
    }

    pub fn call(&self, arguments: &[crate::Value]) -> Result<crate::Value, crate::Failure> {
        self.0.check(arguments)?;
        self.0.call(arguments)
    }

    pub fn lazy(&self) -> bool {
        self.0.lazy()
    }
}
